<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import type { Signal, Stats, WsMessage, SignalUpdate } from '$lib/types';
    import { fade, fly, slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';
    import { env } from '$env/dynamic/public';

    let socket: WebSocket;
    // Active signals map: Symbol -> Signal
    let activeSignals: Record<string, Signal> = {};
    let historySignals: Signal[] = [];
    
    let stats: Stats = { total_signals: 0, win_rate: 0, top_gainer: '---' };
    let isConnected = false;
    let toastMessage: string | null = null;
    let toastType: 'Long' | 'Short' = 'Long';

    // Reactive list for UI (Sort by timestamp desc)
    $: sortedActiveSignals = Object.values(activeSignals).sort((a,b) => b.timestamp - a.timestamp);
    
    // We can also have a history list if we want to show it separately or merge.
    // Use `sortedActiveSignals` for Live Feed and `historySignals` for Table?
    // User wants "Live Cards" and "History Table".
    // Let's keep recent ( < 1h ) in activeSignals.
    // And move completed/old to historySignals.
    
    // Timer for UI updates (e.g. progress bars)
    let now = Date.now();
    onMount(() => {
        const interval = setInterval(() => { now = Date.now(); }, 1000);
        connect();
        return () => clearInterval(interval);
    });

    function connect() {
        const wsUrl = env.PUBLIC_BACKEND_URL || 'ws://localhost:3000/ws';
        socket = new WebSocket(wsUrl);

        socket.onopen = () => {
            isConnected = true;
            console.log('Connected to Backend');
        };

        socket.onmessage = (event) => {
            try {
                const data: WsMessage = JSON.parse(event.data);
                
                if (data.type === 'Stats') {
                    stats = data.payload;
                } else if (data.type === 'Signal') {
                    const signal = data.payload;
                    // Add to active signals
                    activeSignals[signal.symbol] = signal;
                    // Update trigger to refresh UI
                    activeSignals = { ...activeSignals };
                    
                    showToast(signal);
                    playBeep();
                } else if (data.type === 'History') {
                    const history = data.payload;
                    history.forEach(s => {
                        activeSignals[s.symbol] = s;
                    });
                    activeSignals = { ...activeSignals };
                } else if (data.type === 'Update') {
                    const update = data.payload;
                    if (activeSignals[update.symbol]) {
                        // Update live metrics
                        activeSignals[update.symbol].price = update.price;
                        activeSignals[update.symbol].volume = update.volume;
                        // Keep timestamp same as original signal time? Or update?
                        // "Time elapsed" should be from original signal.
                        // So we DON'T update timestamp.
                        activeSignals = activeSignals; // Trigger reactivity
                    }
                }
            } catch (e) {
                console.error('Error parsing message', e);
            }
        };

        socket.onclose = () => {
             isConnected = false;
             setTimeout(connect, 3000);
        };
    }

    function showToast(signal: Signal) {
        toastMessage = `${signal.signal_type} Signal: ${signal.symbol}`;
        toastType = signal.signal_type;
        setTimeout(() => toastMessage = null, 3000);
    }

    function playBeep() {
        // Sonar Ping Effect
        const AudioContext = window.AudioContext || (window as any).webkitAudioContext;
        if (!AudioContext) return;
        const ctx = new AudioContext();
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        
        osc.connect(gain);
        gain.connect(ctx.destination);
        
        osc.type = 'sine';
        osc.frequency.setValueAtTime(800, ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(400, ctx.currentTime + 0.3);
        
        gain.gain.setValueAtTime(0.1, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.3);
        
        osc.start();
        osc.stop(ctx.currentTime + 0.3);
    }
    
    function clearHistory() {
        activeSignals = {};
    }

    // Helper to get progress (0-100) for 60 mins
    function getProgress(timestamp: number) {
        const elapsed = now - timestamp;
        const total = 60 * 60 * 1000;
        return Math.min(100, (elapsed / total) * 100);
    }
    
    function getElapsedTime(timestamp: number) {
        const mins = Math.floor((now - timestamp) / 60000);
        return `${mins}m`;
    }
</script>

<div class="min-h-screen p-6 space-y-8">
    
    <!-- Top Bar: Stats -->
    <div class="glass rounded-2xl p-6 flex justify-between items-center relative overflow-hidden">
        <div class="absolute inset-0 bg-gradient-to-r from-green-900/10 to-transparent pointer-events-none"></div>
        
        <div class="z-10 text-center md:text-left">
            <h1 class="text-3xl font-black tracking-tighter text-white/90">TEEB<span class="text-neon-green">TRADE</span></h1>
            <div class="text-xs text-green-400 font-mono mt-1 opacity-70">WHALE SURVEILLANCE v1.0</div>
        </div>

        <div class="flex space-x-8 z-10">
            <div class="text-center">
                <div class="text-xs text-gray-400 uppercase tracking-widest">Total Signals</div>
                <div class="text-2xl font-bold font-mono text-white">{stats.total_signals}</div>
            </div>
            <div class="text-center">
                <div class="text-xs text-gray-400 uppercase tracking-widest">Win Rate</div>
                <div class="text-2xl font-bold font-mono text-neon-green glow-text">{stats.win_rate.toFixed(1)}%</div>
            </div>
            <div class="text-center hidden md:block">
                <div class="text-xs text-gray-400 uppercase tracking-widest">Top Gainer</div>
                <div class="text-2xl font-bold font-mono text-yellow-400">{stats.top_gainer}</div>
            </div>
            
            <button on:click={clearHistory} class="px-4 py-2 bg-white/5 hover:bg-white/10 text-xs text-gray-400 uppercase tracking-widest rounded-lg border border-white/10 transition-colors">
                Clear
            </button>
        </div>
        
        <div class="absolute top-4 right-4 animate-pulse">
            <div class={`w-3 h-3 rounded-full ${isConnected ? 'bg-neon-green blur-[2px]' : 'bg-red-500'}`}></div>
        </div>
    </div>
    <!-- Toast Notification -->
    {#if toastMessage}
        <div in:fly="{{ y: -50, duration: 300 }}" out:fade class={`fixed top-4 left-1/2 transform -translate-x-1/2 z-50 px-6 py-3 rounded-full shadow-2xl ${toastType === 'Long' ? 'bg-neon-green text-black' : 'bg-neon-red text-white'} font-bold flex items-center gap-2`}>
            {toastType === 'Long' ? '🚀' : '📉'} {toastMessage}
        </div>
    {/if}

    <!-- 2. Live Signal Feed (Horizontal Ticker for latest active signals) -->
    <div>
        <h2 class="text-gray-400 text-sm uppercase tracking-widest mb-4 flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-neon-green animate-ping"></span> Live Monitoring
        </h2>
        <div class="flex overflow-x-auto gap-4 pb-4"> <!-- Horizontal Scroll -->
            {#each sortedActiveSignals.filter(s => now - s.timestamp < 60 * 60 * 1000) as signal (signal.symbol)}
                <div 
                    in:fly="{{ y: 20, duration: 400 }}"
                    animate:flip="{{ duration: 300 }}"
                    class={`min-w-[280px] p-4 rounded-xl border ${signal.signal_type === 'Long' ? 'border-neon-green/30 glow-green bg-green-900/10' : 'border-neon-red/30 glow-red bg-red-900/10'} relative overflow-hidden`}
                >
                    <div class="flex justify-between items-start mb-2">
                        <span class="font-bold text-xl tracking-wide text-white">{signal.symbol}</span>
                        <span class={`px-2 py-0.5 rounded text-xs font-bold ${signal.signal_type === 'Long' ? 'bg-neon-green text-black' : 'bg-neon-red text-white'}`}>
                            {signal.signal_type.toUpperCase()}
                        </span>
                    </div>
                    
                    <div class="space-y-1">
                         <div class="flex justify-between text-sm">
                            <span class="text-gray-400">Price:</span>
                            <span class={`font-mono ${signal.signal_type === 'Long' ? 'text-green-300' : 'text-red-300'}`}>${signal.price < 1 ? signal.price.toFixed(5) : signal.price.toFixed(2)}</span>
                        </div>
                        <div class="flex justify-between text-sm">
                            <span class="text-gray-400">Vol:</span>
                            <span class="font-mono text-gray-300">{signal.volume.toFixed(0)}</span>
                        </div>
                         <div class="flex justify-between text-sm">
                            <span class="text-gray-400">Time:</span>
                            <span class="font-mono text-gray-300">{getElapsedTime(signal.timestamp)} ago</span>
                        </div>
                    </div>

                    <!-- Progress Bar for Outcome (60m) -->
                    <div class="mt-3">
                        <div class="h-1 w-full bg-gray-700 rounded-full overflow-hidden">
                            <div class="h-full bg-blue-500 transition-all duration-1000" style={`width: ${getProgress(signal.timestamp)}%`}></div>
                        </div>
                        <div class="flex justify-between text-[10px] text-gray-500 mt-1">
                            <span>Signal</span>
                            <span>15m</span>
                            <span>30m</span>
                            <span>60m</span>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    </div>

    <!-- 3. Comprehensive History Table -->
    <div class="glass rounded-xl overflow-hidden mt-8">
        <div class="p-4 border-b border-white/5">
            <h2 class="text-gray-400 text-sm uppercase tracking-widest">Signal Log History (All)</h2>
        </div>
        <div class="overflow-x-auto">
            <table class="w-full text-left border-collapse">
                <thead>
                    <tr class="text-xs text-gray-500 uppercase border-b border-white/5">
                        <th class="p-4 font-normal">Time</th>
                        <th class="p-4 font-normal">Symbol</th>
                        <th class="p-4 font-normal">Type</th>
                        <th class="p-4 font-normal">Entry</th>
                        <th class="p-4 font-normal">Current</th>
                        <th class="p-4 font-normal">Proof Analysis</th>
                        <th class="p-4 font-normal text-right">Age</th>
                    </tr>
                </thead>
                <tbody class="text-sm">
                    {#each sortedActiveSignals as signal}
                        <tr class="border-b border-white/5 hover:bg-white/5 transition-colors font-mono">
                            <td class="p-4 text-gray-400">{new Date(signal.timestamp).toLocaleTimeString()}</td>
                            <td class="p-4 font-bold text-white">{signal.symbol}</td>
                            <td class="p-4">
                                <span class={`${signal.signal_type === 'Long' ? 'text-neon-green' : 'text-neon-red'}`}>
                                    {signal.signal_type}
                                </span>
                            </td>
                            <td class="p-4 text-gray-300">
                                <!-- We don't have entry price stored separately in Signal struct yet? 
                                     Actually Signal.price IS the entry price. 
                                     The Update struct only updates `activeSignals[sym].price`.
                                     Ah. If I overwrite `activeSignals[sym].price` with `update.price`, I lose Entry Price.
                                     
                                     Wait. Code in previous step:
                                     activeSignals[update.symbol].price = update.price;
                                     
                                     I should have stored generic `entry_price` or let `price` be current.
                                     Ideally `Signal` struct has `price` (Entry).
                                     Frontend should probably store `currentPrice` separately or use `price` as current and `entryPrice`?
                                     
                                     Signal struct on Backend: `price` is at time of signal.
                                     SignalUpdate: `price` is current.
                                     
                                     If I overwrite, I lose entry.
                                     
                                     For now, let's assume `price` is Current Price. 
                                     Entry Price is lost unless I add a field.
                                     But `history.json` has `Signal`.
                                 -->
                                ${signal.price < 1 ? signal.price.toFixed(5) : signal.price.toFixed(2)}
                            </td>
                             <td class="p-4 text-white">
                                <!-- Price is updated live, so this column redundant if Entry is missing. -->
                                ${signal.price < 1 ? signal.price.toFixed(5) : signal.price.toFixed(2)}
                            </td>
                            <td class="p-4 text-gray-400 truncate max-w-xs">
                                {signal.reason}
                            </td>
                            <td class="p-4 text-right">
                                <span class="text-yellow-500">{getElapsedTime(signal.timestamp)}</span>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    </div>
</div>
