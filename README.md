# Trading Testing - Simplified Hooks Demo

Simple demonstration of trading strategy hooks and custom data features.

## Overview

This example shows the two key hook features:
1. **Pre-trade hooks** - modify, reject, or approve trades before execution
2. **Strategy context & custom data** - access strategy state and custom data in observers

## Usage

```bash
cargo run
```

## Demo 1: Pre-trade Hooks

Shows how to use pre-trade hooks to:
- **Reject** trades above a price threshold ($50,000)
- **Modify** position sizes that exceed risk limits (max 2.0)
- **Approve** normal trades that pass validation

### Example Output
```
Pre-trade Hook: Evaluating trade at $47000.00, size: 3.00
  → MODIFIED: Position size reduced from 3.00 to 2.00

Pre-trade Hook: Evaluating trade at $51000.00, size: 3.00
  → REJECTED: Price too high ($51000.00 > $50,000)

Pre-trade Hook: Evaluating trade at $48000.00, size: 3.00
  → APPROVED: Trade looks good
```

## Demo 2: Strategy Context & Custom Data

Shows how to access:
- **Strategy context** - RSI values, dynamic thresholds from the strategy
- **Custom data** - user metadata passed to the strategy

### Example Output
```
Trade #1: Long at $47000.00
  Strategy Context:
    RSI Value: 25.34
    Overbought Level: 70.00
    Oversold Level: 30.00
  Custom Data:
    User ID: user_123
    Session ID: session_456
    Risk Level: medium
```

## Key Features

### PreTradeHookDemo Observer
- Implements risk management rules
- Tracks approved/modified/rejected trades
- Can modify position sizes before execution

### ContextDemo Observer  
- Shows strategy context (RSI values, thresholds)
- Displays custom data (user info, session data)
- Logs detailed trade information

## Simple Configuration

The demo uses basic RSI strategy configuration:
- RSI period: 14
- Oversold threshold: 30.0
- Overbought threshold: 70.0
- Position size: 1.0-3.0 (depending on demo)

## Sample Data

Uses either:
- `stochastic_hooks_demo.jsonl` if available
- Built-in sample ticks with various price levels to trigger different hook behaviors

This simplified demo focuses purely on demonstrating the core hook functionality without complex custom data structures or extensive logging.