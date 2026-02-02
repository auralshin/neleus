//! Data formatters for LLM consumption.
//!
//! These formatters convert trading data into formats optimized for LLM understanding.

use serde_json::Value;

/// Format market data for LLM consumption.
pub struct MarketDataFormatter;

impl MarketDataFormatter {
    /// Format ticker data as natural language.
    pub fn format_ticker(data: &Value) -> String {
        let price = data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let volume_24h = data.get("volume_24h").and_then(|v| v.as_f64());
        let change_24h = data.get("change_24h").and_then(|v| v.as_f64());
        let high_24h = data.get("high_24h").and_then(|v| v.as_f64());
        let low_24h = data.get("low_24h").and_then(|v| v.as_f64());
        let instrument = data
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let mut text = format!("**{}**: Current Price: ${:.2}", instrument, price);

        if let Some(change) = change_24h {
            let direction = if change >= 0.0 { "up" } else { "down" };
            text.push_str(&format!(" ({} {:.2}% in 24h)", direction, change.abs()));
        }

        if let (Some(high), Some(low)) = (high_24h, low_24h) {
            text.push_str(&format!("\n  24h Range: ${:.2} - ${:.2}", low, high));
        }

        if let Some(volume) = volume_24h {
            text.push_str(&format!("\n  24h Volume: ${:.0}", volume));
        }

        text
    }

    /// Format ticker data as JSON.
    pub fn format_ticker_json(data: &Value) -> Value {
        // Pass through, ensuring consistent structure
        serde_json::json!({
            "instrument": data.get("instrument"),
            "price": data.get("price"),
            "volume_24h": data.get("volume_24h"),
            "change_24h": data.get("change_24h"),
            "high_24h": data.get("high_24h"),
            "low_24h": data.get("low_24h"),
            "bid": data.get("bid"),
            "ask": data.get("ask"),
            "timestamp": data.get("timestamp"),
        })
    }

    /// Format orderbook for LLM.
    pub fn format_orderbook(data: &Value) -> String {
        let instrument = data
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let bids = data.get("bids").and_then(|v| v.as_array());
        let asks = data.get("asks").and_then(|v| v.as_array());

        let mut text = format!("**{} Orderbook**\n", instrument);

        if let Some(asks) = asks {
            text.push_str("Asks (sell orders, lowest first):\n");
            for (i, ask) in asks.iter().take(5).enumerate() {
                let price = ask.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let size = ask.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                text.push_str(&format!("  {}: ${:.2} x {:.4}\n", i + 1, price, size));
            }
        }

        if let Some(bids) = bids {
            text.push_str("Bids (buy orders, highest first):\n");
            for (i, bid) in bids.iter().take(5).enumerate() {
                let price = bid.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let size = bid.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                text.push_str(&format!("  {}: ${:.2} x {:.4}\n", i + 1, price, size));
            }
        }

        // Calculate spread
        if let (Some(asks), Some(bids)) = (asks, bids) {
            if let (Some(best_ask), Some(best_bid)) = (asks.first(), bids.first()) {
                let ask_price = best_ask.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let bid_price = best_bid.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let spread = ask_price - bid_price;
                let spread_pct = if bid_price > 0.0 {
                    (spread / bid_price) * 100.0
                } else {
                    0.0
                };
                text.push_str(&format!(
                    "Spread: ${:.2} ({:.4}%)",
                    spread, spread_pct
                ));
            }
        }

        text
    }

    /// Format candles/OHLCV data.
    pub fn format_candles(data: &Value, limit: usize) -> String {
        let instrument = data
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let candles = data.get("candles").and_then(|v| v.as_array());

        let mut text = format!("**{} Recent Candles**\n", instrument);

        if let Some(candles) = candles {
            text.push_str("Time | Open | High | Low | Close | Volume\n");
            text.push_str("-----|------|------|-----|-------|-------\n");

            for candle in candles.iter().take(limit) {
                let timestamp = candle.get("timestamp").and_then(|v| v.as_str()).unwrap_or("-");
                let open = candle.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let high = candle.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let low = candle.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let close = candle.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let volume = candle.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);

                text.push_str(&format!(
                    "{} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2}\n",
                    timestamp, open, high, low, close, volume
                ));
            }
        }

        text
    }
}

/// Format trading signals for LLM consumption.
pub struct SignalFormatter;

impl SignalFormatter {
    /// Format a signal as natural language.
    pub fn format_signal(data: &Value) -> String {
        let instrument = data
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let direction = data
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("neutral");
        let strength = data.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let signal_type = data
            .get("signal_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let source = data.get("source").and_then(|v| v.as_str());

        let strength_word = if strength > 0.8 {
            "Strong"
        } else if strength > 0.5 {
            "Moderate"
        } else {
            "Weak"
        };

        let mut text = format!(
            "**{} Signal**: {} {} (strength: {:.0}%)",
            signal_type.to_uppercase(),
            strength_word,
            direction.to_uppercase(),
            strength * 100.0
        );

        text.push_str(&format!("\n  Instrument: {}", instrument));

        if let Some(src) = source {
            text.push_str(&format!("\n  Source: {}", src));
        }

        // Add confidence interpretation
        let action_suggestion = match (direction, strength > 0.6) {
            ("long" | "buy", true) => "Consider entering a long position",
            ("short" | "sell", true) => "Consider entering a short position or closing longs",
            ("long" | "buy", false) => "Weak buy signal - wait for confirmation",
            ("short" | "sell", false) => "Weak sell signal - wait for confirmation",
            _ => "No clear directional bias",
        };

        text.push_str(&format!("\n  Suggested Action: {}", action_suggestion));

        text
    }

    /// Format multiple signals as a summary.
    pub fn format_signals_summary(signals: &[Value]) -> String {
        if signals.is_empty() {
            return "No active signals".to_string();
        }

        let mut text = format!("**Active Signals ({} total)**\n\n", signals.len());

        // Group by direction
        let mut longs = Vec::new();
        let mut shorts = Vec::new();
        let mut neutrals = Vec::new();

        for signal in signals {
            let direction = signal
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("neutral");
            match direction {
                "long" | "buy" => longs.push(signal),
                "short" | "sell" => shorts.push(signal),
                _ => neutrals.push(signal),
            }
        }

        if !longs.is_empty() {
            text.push_str(&format!("🟢 Long signals: {}\n", longs.len()));
        }
        if !shorts.is_empty() {
            text.push_str(&format!("🔴 Short signals: {}\n", shorts.len()));
        }
        if !neutrals.is_empty() {
            text.push_str(&format!("⚪ Neutral signals: {}\n", neutrals.len()));
        }

        // Calculate overall sentiment
        let sentiment = if longs.len() > shorts.len() * 2 {
            "Bullish"
        } else if shorts.len() > longs.len() * 2 {
            "Bearish"
        } else {
            "Mixed"
        };

        text.push_str(&format!("\nOverall Sentiment: {}", sentiment));

        text
    }
}

/// Format portfolio data for LLM consumption.
pub struct PortfolioFormatter;

impl PortfolioFormatter {
    /// Format portfolio overview.
    pub fn format_portfolio(data: &Value) -> String {
        let equity = data.get("equity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let unrealized_pnl = data
            .get("unrealized_pnl")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let realized_pnl = data.get("realized_pnl").and_then(|v| v.as_f64());
        let margin_used = data.get("margin_used").and_then(|v| v.as_f64());
        let positions = data.get("positions").and_then(|v| v.as_array());

        let mut text = String::from("**Portfolio Summary**\n");
        text.push_str(&format!("  Total Equity: ${:.2}\n", equity));

        let pnl_emoji = if unrealized_pnl >= 0.0 { "📈" } else { "📉" };
        text.push_str(&format!(
            "  Unrealized P&L: {} ${:+.2}\n",
            pnl_emoji, unrealized_pnl
        ));

        if let Some(realized) = realized_pnl {
            text.push_str(&format!("  Realized P&L: ${:+.2}\n", realized));
        }

        if let Some(margin) = margin_used {
            let margin_pct = if equity > 0.0 {
                (margin / equity) * 100.0
            } else {
                0.0
            };
            text.push_str(&format!(
                "  Margin Used: ${:.2} ({:.1}%)\n",
                margin, margin_pct
            ));
        }

        if let Some(positions) = positions {
            text.push_str(&format!("\n**Open Positions ({})**\n", positions.len()));

            for pos in positions {
                let instrument = pos
                    .get("instrument")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let side = pos.get("side").and_then(|v| v.as_str()).unwrap_or("unknown");
                let size = pos.get("size").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let entry_price = pos.get("entry_price").and_then(|v| v.as_f64());
                let current_price = pos.get("current_price").and_then(|v| v.as_f64());
                let pnl = pos.get("unrealized_pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let side_emoji = if side == "long" { "🟢" } else { "🔴" };
                text.push_str(&format!(
                    "  {} {} {} x {:.4}",
                    side_emoji,
                    instrument,
                    side.to_uppercase(),
                    size.abs()
                ));

                if let Some(entry) = entry_price {
                    text.push_str(&format!(" @ ${:.2}", entry));
                }

                if let Some(current) = current_price {
                    text.push_str(&format!(" (now ${:.2})", current));
                }

                let pnl_emoji = if pnl >= 0.0 { "+" } else { "" };
                text.push_str(&format!(" P&L: {}${:.2}\n", pnl_emoji, pnl));
            }
        } else {
            text.push_str("\nNo open positions.");
        }

        text
    }

    /// Format as JSON.
    pub fn format_portfolio_json(data: &Value) -> Value {
        data.clone()
    }
}

/// Format technical analysis for LLM consumption.
pub struct AnalysisFormatter;

impl AnalysisFormatter {
    /// Format full analysis report.
    pub fn format_full_analysis(data: &Value) -> String {
        let instrument = data
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let overall_signal = data
            .get("overall_signal")
            .and_then(|v| v.as_str())
            .unwrap_or("neutral");
        let indicators = data.get("indicators");

        let signal_emoji = match overall_signal {
            "buy" | "strong_buy" => "🟢",
            "sell" | "strong_sell" => "🔴",
            _ => "⚪",
        };

        let mut text = format!(
            "**{} Technical Analysis**\n\nOverall Signal: {} {}\n\n",
            instrument,
            signal_emoji,
            overall_signal.to_uppercase().replace('_', " ")
        );

        if let Some(indicators) = indicators {
            text.push_str("**Indicators:**\n");

            // RSI
            if let Some(rsi) = indicators.get("rsi").and_then(|v| v.as_f64()) {
                let rsi_signal = if rsi < 30.0 {
                    "OVERSOLD (bullish)"
                } else if rsi > 70.0 {
                    "OVERBOUGHT (bearish)"
                } else {
                    "neutral"
                };
                text.push_str(&format!("  RSI(14): {:.1} - {}\n", rsi, rsi_signal));
            }

            // MACD
            if let Some(macd) = indicators.get("macd") {
                let value = macd.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let signal_line = macd.get("signal").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let histogram = macd.get("histogram").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let macd_signal = if histogram > 0.0 {
                    "bullish momentum"
                } else {
                    "bearish momentum"
                };

                text.push_str(&format!(
                    "  MACD: {:.4} (signal: {:.4}, hist: {:.4}) - {}\n",
                    value, signal_line, histogram, macd_signal
                ));
            }

            // Bollinger Bands
            if let Some(bb) = indicators.get("bollinger") {
                let upper = bb.get("upper").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let middle = bb.get("middle").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let lower = bb.get("lower").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let current_price = data.get("price").and_then(|v| v.as_f64());

                text.push_str(&format!(
                    "  Bollinger Bands: Upper ${:.2}, Middle ${:.2}, Lower ${:.2}\n",
                    upper, middle, lower
                ));

                if let Some(price) = current_price {
                    let position = if price > upper {
                        "above upper band (overbought)"
                    } else if price < lower {
                        "below lower band (oversold)"
                    } else if price > middle {
                        "above middle band (bullish bias)"
                    } else {
                        "below middle band (bearish bias)"
                    };
                    text.push_str(&format!("    Price is {}\n", position));
                }
            }

            // Moving Averages
            if let Some(ma) = indicators.get("moving_averages") {
                text.push_str("  Moving Averages:\n");
                
                if let Some(sma20) = ma.get("sma_20").and_then(|v| v.as_f64()) {
                    text.push_str(&format!("    SMA(20): ${:.2}\n", sma20));
                }
                if let Some(sma50) = ma.get("sma_50").and_then(|v| v.as_f64()) {
                    text.push_str(&format!("    SMA(50): ${:.2}\n", sma50));
                }
                if let Some(ema12) = ma.get("ema_12").and_then(|v| v.as_f64()) {
                    text.push_str(&format!("    EMA(12): ${:.2}\n", ema12));
                }
            }
        }

        // Support/Resistance
        if let Some(levels) = data.get("support_resistance") {
            text.push_str("\n**Key Levels:**\n");
            
            if let Some(resistance) = levels.get("resistance").and_then(|v| v.as_array()) {
                let levels_str: Vec<String> = resistance
                    .iter()
                    .take(3)
                    .filter_map(|v| v.as_f64())
                    .map(|v| format!("${:.2}", v))
                    .collect();
                if !levels_str.is_empty() {
                    text.push_str(&format!("  Resistance: {}\n", levels_str.join(", ")));
                }
            }

            if let Some(support) = levels.get("support").and_then(|v| v.as_array()) {
                let levels_str: Vec<String> = support
                    .iter()
                    .take(3)
                    .filter_map(|v| v.as_f64())
                    .map(|v| format!("${:.2}", v))
                    .collect();
                if !levels_str.is_empty() {
                    text.push_str(&format!("  Support: {}\n", levels_str.join(", ")));
                }
            }
        }

        text
    }

    /// Format as JSON.
    pub fn format_analysis_json(data: &Value) -> Value {
        data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ticker() {
        let data = serde_json::json!({
            "instrument": "BTC-PERP",
            "price": 50000.0,
            "change_24h": 2.5,
            "high_24h": 51000.0,
            "low_24h": 49000.0,
            "volume_24h": 1000000000.0
        });

        let text = MarketDataFormatter::format_ticker(&data);
        assert!(text.contains("BTC-PERP"));
        assert!(text.contains("50000"));
        assert!(text.contains("up 2.5%"));
    }

    #[test]
    fn test_format_signal() {
        let data = serde_json::json!({
            "instrument": "ETH-PERP",
            "direction": "long",
            "strength": 0.85,
            "signal_type": "momentum",
            "source": "RSI Strategy"
        });

        let text = SignalFormatter::format_signal(&data);
        assert!(text.contains("Strong"));
        assert!(text.contains("LONG"));
        assert!(text.contains("85%"));
    }

    #[test]
    fn test_format_portfolio() {
        let data = serde_json::json!({
            "equity": 100000.0,
            "unrealized_pnl": 500.0,
            "positions": [
                {
                    "instrument": "BTC-PERP",
                    "side": "long",
                    "size": 0.5,
                    "entry_price": 50000.0,
                    "unrealized_pnl": 500.0
                }
            ]
        });

        let text = PortfolioFormatter::format_portfolio(&data);
        assert!(text.contains("100000"));
        assert!(text.contains("BTC-PERP"));
        assert!(text.contains("LONG"));
    }
}
