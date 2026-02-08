export enum SignalType {
    Long = "Long",
    Short = "Short",
}

export interface Signal {
    symbol: string;
    signal_type: SignalType;
    price: number;
    volume: number;
    avg_volume: number;
    timestamp: number;
    reason: string;
    // Optional proofs
    order_book_ratio?: number;
    oi?: number;
    net_inflow?: number;
}

export interface SignalUpdate {
    symbol: string;
    price: number;
    volume: number;
    timestamp: number;
}

export type WsMessage =
    | { type: 'Signal', payload: Signal }
    | { type: 'Update', payload: SignalUpdate }
    | { type: 'Stats', payload: Stats }
    | { type: 'History', payload: Signal[] };

export interface Stats {
    total_signals: number;
    win_rate: number;
    top_gainer: string;
}
