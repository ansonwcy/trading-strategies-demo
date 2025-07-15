# Trading Testing

Simple test runner for the stochastic trading strategy.

## Usage

```bash
cargo run
```

## What it does

1. Loads tick data from `tick_strategy_ticks.jsonl`
2. Converts ticks to 1-second candles
3. Runs the stochastic strategy
4. Shows trades and final P&L

## Configuration

Edit `src/main.rs` to change:
- `timeframe_secs` - candle size (default: 1 second)
- `k_period` - stochastic K period (default: 5)
- `oversold_threshold` - buy when K < this (default: 20)
- `overbought_threshold` - sell when K > this (default: 80)