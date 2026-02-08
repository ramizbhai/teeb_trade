<script>
    import "../app.css";
</script>

<div class="bg-gray-900 min-h-screen text-white font-sans">
    <header class="p-4 bg-gray-800 border-b border-gray-700 flex justify-between items-center">
        <h1 class="text-2xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent">Teeb Trade Scanner</h1>
        <div class="text-sm text-gray-400">Phase 1: Whale Volume Scanner</div>
    </header>
    <main class="container mx-auto p-4">
        <slot />
    </main>
</div>
