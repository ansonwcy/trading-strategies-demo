# Trading Testing - Observer Context Demo

Demonstrates the new observer context feature in the trading strategies library.

## Overview

This example shows how trading strategies can pass custom context data to observers, allowing for more sophisticated trade monitoring and risk management.

## Usage

```bash
cargo run
```

## What it demonstrates

### 1. Basic Observer Pattern (Stochastic Strategy)
- Shows traditional observer pattern without context
- Implements pre-trade validation and post-trade logging
- Demonstrates trade modification based on risk limits

### 2. Context-Aware Observers (RSI Strategy)
- Shows how strategies can pass custom context data
- RSI strategy passes `RsiTradeContext` containing:
  - Current RSI value
  - Dynamic overbought threshold
  - Dynamic oversold threshold
- Observer can access this data to make informed decisions

## Key Features Demonstrated

### Pre-trade Hooks
- Validate trades before execution
- Reject trades based on price limits
- Modify position sizes for risk management
- Access strategy-specific context for advanced validation

### Post-trade Hooks
- Log executed trades
- Track P&L and performance metrics
- Access context data for detailed analysis

## Configuration

Edit `src/main.rs` to change strategy parameters:

### Stochastic Strategy
- `k_period` - stochastic K period (default: 14)
- `d_period` - smoothing period (default: 3)
- `oversold_threshold` - buy signal threshold (default: 20)
- `overbought_threshold` - sell signal threshold (default: 80)

### RSI Strategy
- `rsi_period` - RSI calculation period (default: 14)
- `oversold_threshold` - buy signal threshold (default: 30)
- `overbought_threshold` - sell signal threshold (default: 70)
- `use_dynamic_levels` - enable adaptive thresholds (default: true)

## Example Output

```
🔎 [CONTEXT-AWARE OBSERVER] Pre-trade analysis
   Proposed: BUY 1.0 units at $2850.50
   📊 RSI Strategy Context:
     - Current RSI: 28.45
     - Dynamic Oversold: 32.10
     - Dynamic Overbought: 67.90
     ⚠️  WARNING: Extremely oversold (RSI < 30)
```

## Use Cases

1. **Risk Management**: Use context to implement sophisticated risk rules
2. **Performance Analysis**: Log strategy-specific metrics with each trade
3. **Compliance**: Ensure trades meet regulatory requirements based on market conditions
4. **Strategy Optimization**: Collect detailed data for backtesting improvements