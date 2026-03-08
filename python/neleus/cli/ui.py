"""
Rich terminal UI helpers for the Neleus CLI.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Deque, Dict, Iterable, List, Optional

from rich import box
from rich.align import Align
from rich.columns import Columns
from rich.console import Group
from rich.layout import Layout
from rich.panel import Panel
from rich.table import Table
from rich.text import Text
from rich.tree import Tree

BRAND_ART = r"""
 _   _ _____ _     _____ _   _ ____  
| \ | | ____| |   | ____| | | / ___| 
|  \| |  _| | |   |  _| | | | \___ \ 
| |\  | |___| |___| |___| |_| |___) |
|_| \_|_____|_____|_____|\___/|____/ 
"""


def _parse_iso_datetime(value: str) -> Optional[datetime]:
    if not value:
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def _format_timestamp(value: str) -> str:
    parsed = _parse_iso_datetime(value)
    if parsed is None:
        return "-"
    return parsed.astimezone(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")


def _format_short_timestamp(value: str) -> str:
    parsed = _parse_iso_datetime(value)
    if parsed is None:
        return "-"
    return parsed.astimezone(timezone.utc).strftime("%H:%M:%S")


def _format_epoch_ms(value: int) -> str:
    return datetime.fromtimestamp(value / 1000.0, tz=timezone.utc).strftime("%H:%M:%S UTC")


def _format_duration(start: str, end: str) -> str:
    start_dt = _parse_iso_datetime(start)
    end_dt = _parse_iso_datetime(end)
    if start_dt is None or end_dt is None:
        return "-"

    total_seconds = max(int((end_dt - start_dt).total_seconds()), 0)
    hours, remainder = divmod(total_seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    if hours:
        return f"{hours}h {minutes}m {seconds}s"
    if minutes:
        return f"{minutes}m {seconds}s"
    return f"{seconds}s"


def _format_signed_pct(value: float) -> str:
    return f"{value:+.2f}%"


def _format_currency(value: float) -> str:
    return f"${value:,.2f}"


def _format_price(value: float) -> str:
    return f"{value:,.4f}"


def _market_bias_style(bias: str) -> str:
    return {
        "long": "green",
        "short": "red",
        "neutral": "yellow",
    }.get(str(bias).lower(), "cyan")


def _trend_style(trend: str) -> str:
    return {
        "bullish": "green",
        "bearish": "red",
        "sideways": "yellow",
    }.get(str(trend).lower(), "cyan")


def _momentum_style(momentum: str) -> str:
    return {
        "rising": "green",
        "fading": "yellow",
        "oversold": "cyan",
        "overbought": "magenta",
    }.get(str(momentum).lower(), "cyan")


def _signed_style(value: float) -> str:
    if value > 0:
        return "green"
    if value < 0:
        return "red"
    return "yellow"


def _metric_card(label: str, value: str, style: str, subtitle: Optional[str] = None) -> Panel:
    text = Text(justify="center")
    text.append(f"{value}\n", style=f"bold {style}")
    text.append(label, style="dim")
    if subtitle:
        text.append(f"\n{subtitle}", style="dim")
    return Panel(Align.center(text), border_style=style, padding=(0, 1))


def _kv_table(title: str, rows: Iterable[tuple[str, str]]) -> Panel:
    table = Table(box=box.SIMPLE_HEAVY, expand=True, show_header=False, pad_edge=False)
    table.add_column("Key", style="cyan", no_wrap=True)
    table.add_column("Value")
    for key, value in rows:
        table.add_row(key, value)
    return Panel(table, title=title, border_style="bright_blue")


def _orders_table(orders: List[Dict[str, Any]], title: str, limit: int = 8) -> Panel:
    table = Table(box=box.SIMPLE_HEAVY, expand=True)
    table.add_column("Time", style="dim", no_wrap=True)
    table.add_column("Instrument", style="cyan", no_wrap=True)
    table.add_column("Side", no_wrap=True)
    table.add_column("Type", no_wrap=True)
    table.add_column("Qty", justify="right")
    table.add_column("Price", justify="right")

    recent_orders = list(reversed(orders[-limit:]))
    for order in recent_orders:
        table.add_row(
            str(order.get("timestamp", "-")),
            str(order.get("instrument", "-")),
            str(order.get("side", "-")),
            str(order.get("order_type", "-")),
            f'{float(order.get("quantity", 0.0)):.6f}',
            "-" if order.get("price") is None else f'{float(order["price"]):.4f}',
        )

    if not recent_orders:
        table.add_row("-", "-", "-", "-", "-", "-")

    return Panel(table, title=title, border_style="bright_blue")


def _fills_table(fills: List[Dict[str, Any]], title: str, limit: int = 8) -> Panel:
    table = Table(box=box.SIMPLE_HEAVY, expand=True)
    table.add_column("Time", style="dim", no_wrap=True)
    table.add_column("Side", no_wrap=True)
    table.add_column("Qty", justify="right")
    table.add_column("Price", justify="right")
    table.add_column("Fee", justify="right")

    recent_fills = list(reversed(fills[-limit:]))
    for fill in recent_fills:
        timestamp = fill.get("timestamp")
        if isinstance(timestamp, (int, float)):
            time_label = datetime.fromtimestamp(float(timestamp) / 1000, tz=timezone.utc).strftime(
                "%Y-%m-%d %H:%M"
            )
        else:
            time_label = str(timestamp or "-")

        table.add_row(
            time_label,
            str(fill.get("side", "-")),
            f'{float(fill.get("quantity", 0.0)):.6f}',
            f'{float(fill.get("price", 0.0)):.4f}',
            f'{float(fill.get("commission", 0.0)):.4f}',
        )

    if not recent_fills:
        table.add_row("-", "-", "-", "-", "-")

    return Panel(table, title=title, border_style="bright_blue")


def render_project_created(
    project_name: str,
    initialized: bool = False,
    db_backend: str = "none",
    trade_monitoring: bool = False,
) -> Panel:
    title = "Project Initialized" if initialized else "Project Ready"
    action = "Initialized" if initialized else "Created"
    body = Text()
    body.append(f"{action} ", style="dim")
    body.append(project_name, style="bold cyan")

    if db_backend != "none":
        body.append("\n\nDatabase adapter: ", style="dim")
        body.append(db_backend.upper(), style="bold magenta")
        if trade_monitoring:
            body.append("  +trade-monitoring", style="bold yellow")
        body.append("\n", style="dim")

    body.append("\n\nNext steps\n", style="bold")
    step = 1
    if not initialized:
        body.append(f"{step}. cd {project_name}\n", style="green")
        step += 1
    body.append(f"{step}. edit neleus.toml  (set [hyperliquid] and [database] sections)\n", style="green")
    step += 1
    if db_backend != "none":
        body.append(f"{step}. neleus db init    (create tables in your database)\n", style="green")
        step += 1
    body.append(f"{step}. neleus market analyze BTC-PERP\n", style="green")
    step += 1
    body.append(f"{step}. neleus backtest --strategy momentum", style="green")
    return Panel(body, title=title, border_style="green", padding=(1, 2))


def render_brand_banner(compact: bool = False) -> Panel:
    art = Text(BRAND_ART.strip("\n"), style="bold cyan")
    links = Text(justify="center")
    links.append("https://neleus.trade", style="bold cyan underline")
    links.append("  |  ", style="dim")
    links.append("Star on GitHub: ", style="dim")
    links.append("https://github.com/auralshin/neleus", style="bold green underline")

    if compact:
        body = Group(Align.center(art), Align.center(links))
    else:
        tagline = Text("Hyperliquid-first trading CLI and Python runtime", style="dim", justify="center")
        body = Group(Align.center(art), Align.center(tagline), Align.center(links))

    return Panel(body, border_style="bright_blue", padding=(1, 2))


def render_about_panel() -> Group:
    command_table = Table(box=box.SIMPLE_HEAVY, expand=True)
    command_table.add_column("Command", style="cyan", no_wrap=True)
    command_table.add_column("What It Does")
    command_table.add_row("neleus market search BTC", "Find matching perp, HIP-3, or spot markets")
    command_table.add_row("neleus market analyze BTC-PERP", "Single-market TA and level read")
    command_table.add_row("neleus market analyze GAS --scope hip3 --dex flx", "Resolve a HIP-3 market by query and analyze it")
    command_table.add_row("neleus market scan --scope perps", "Rank markets by conviction-style TA score")
    command_table.add_row("neleus market book BTC-PERP", "Live L2 order book in the terminal")
    command_table.add_row("neleus market book GAS --scope hip3 --dex flx", "Open a live book without typing the full market id")
    command_table.add_row("neleus new my_project", "Scaffold a strategy project when you need code")

    notes = Text()
    notes.append("Global commands work without creating a project.", style="bold green")
    notes.append("\nProject-only commands are backtest, run, strategy, and project info.", style="dim")

    return Group(
        render_brand_banner(),
        Columns(
            [
                Panel(command_table, title="Quick Commands", border_style="bright_blue"),
                Panel(notes, title="Workflow", border_style="bright_blue", padding=(1, 1)),
            ],
            expand=True,
        ),
    )


def render_market_analysis(analysis: Any) -> Group:
    price_vs_support = ((analysis.last_price / analysis.support) - 1.0) * 100.0 if analysis.support else 0.0
    resistance_room = ((analysis.resistance / analysis.last_price) - 1.0) * 100.0 if analysis.last_price else 0.0

    cards = Columns(
        [
            _metric_card("Last Price", _format_price(analysis.last_price), "cyan", analysis.symbol),
            _metric_card(
                "Price Change",
                _format_signed_pct(analysis.price_change_pct),
                _signed_style(analysis.price_change_pct),
                analysis.timeframe,
            ),
            _metric_card("Volatility", f"{analysis.volatility_pct:.2f}%", "magenta", f"{analysis.candles_analyzed} candles"),
            _metric_card("Bias", str(analysis.bias).upper(), _market_bias_style(analysis.bias), str(analysis.momentum).upper()),
        ],
        expand=True,
        equal=True,
    )

    structure_rows = [
        ("Trend", f"[{_trend_style(analysis.trend)}]{analysis.trend.upper()}[/{_trend_style(analysis.trend)}]"),
        (
            "Momentum",
            f"[{_momentum_style(analysis.momentum)}]{analysis.momentum.upper()}[/{_momentum_style(analysis.momentum)}]",
        ),
        ("RSI", f"{analysis.rsi:.2f}"),
        ("SMA20", _format_price(analysis.sma_fast)),
        ("SMA50", _format_price(analysis.sma_slow)),
        ("EMA12", _format_price(analysis.ema_fast)),
        ("EMA26", _format_price(analysis.ema_slow)),
    ]

    level_rows = [
        ("Support", _format_price(analysis.support)),
        ("Resistance", _format_price(analysis.resistance)),
        ("Bollinger High", _format_price(analysis.bollinger_upper)),
        ("Bollinger Mid", _format_price(analysis.bollinger_mid)),
        ("Bollinger Low", _format_price(analysis.bollinger_lower)),
        ("Pct Above Support", f"{price_vs_support:.2f}%"),
        ("Room To Resistance", f"{resistance_room:.2f}%"),
    ]

    summary = Text()
    summary.append("Trade read\n", style="bold")
    summary.append("Bias ", style="dim")
    summary.append(str(analysis.bias).upper(), style=f"bold {_market_bias_style(analysis.bias)}")
    summary.append(" because price structure is ", style="dim")
    summary.append(str(analysis.trend).upper(), style=f"bold {_trend_style(analysis.trend)}")
    summary.append(" and momentum is ", style="dim")
    summary.append(str(analysis.momentum).upper(), style=f"bold {_momentum_style(analysis.momentum)}")
    summary.append(".\n", style="dim")
    summary.append(f"RSI {analysis.rsi:.2f}", style="cyan")
    summary.append(" | ", style="dim")
    summary.append(f"{price_vs_support:.2f}% above support", style="green")
    summary.append(" | ", style="dim")
    summary.append(f"{resistance_room:.2f}% to resistance", style="yellow")
    summary.append(f"\nGenerated {_format_timestamp(analysis.generated_at)}", style="dim")

    return Group(
        cards,
        Columns(
            [
                _kv_table("Structure", structure_rows),
                _kv_table("Key Levels", level_rows),
                Panel(summary, title="Read", border_style=_market_bias_style(analysis.bias), padding=(1, 1)),
            ],
            expand=True,
        ),
    )


def render_market_catalog(catalog: Any) -> Group:
    scope_label = str(catalog.scope).upper()
    dex_count = len(getattr(catalog, "dex_counts", {}))
    cards = [
        _metric_card("Scope", scope_label, "cyan"),
        _metric_card("Markets", str(catalog.total_markets), "green"),
        _metric_card("Groups", str(dex_count), "magenta"),
    ]
    if getattr(catalog, "scope", "") == "spot":
        cards.append(_metric_card("Tokens", str(getattr(catalog, "total_tokens", 0)), "yellow"))
    else:
        cards.append(_metric_card("Generated", _format_short_timestamp(catalog.generated_at), "yellow"))

    summary_table = Table(box=box.SIMPLE_HEAVY, expand=True)
    summary_table.add_column("Group", style="cyan")
    summary_table.add_column("Markets", justify="right")
    for group, count in sorted(getattr(catalog, "dex_counts", {}).items()):
        summary_table.add_row(str(group), str(count))
    if not getattr(catalog, "dex_counts", {}):
        summary_table.add_row("-", "0")

    table = Table(box=box.SIMPLE_HEAVY, expand=True)
    table.add_column("Name", style="cyan")
    table.add_column("Type", no_wrap=True)
    table.add_column("Group", no_wrap=True)
    table.add_column("Base/Collateral", no_wrap=True)
    table.add_column("Quote/Leverage", no_wrap=True)

    for entry in getattr(catalog, "entries", [])[:80]:
        if entry.market_type == "spot":
            left = entry.base_token or "-"
            right = entry.quote_token or "-"
        else:
            left = entry.collateral_token or "-"
            right = "-" if entry.max_leverage is None else f"{entry.max_leverage}x"
        table.add_row(
            entry.name,
            entry.market_type,
            entry.dex or "-",
            left,
            right,
        )

    if not getattr(catalog, "entries", []):
        table.add_row("-", "-", "-", "-", "-")

    footer = Text()
    footer.append(f"Showing {catalog.total_markets} markets", style="dim")
    if catalog.total_markets > 80:
        footer.append(" (first 80 shown; use --output json for full data)", style="yellow")

    return Group(
        Columns(cards, expand=True, equal=True),
        Columns(
            [
                Panel(summary_table, title="Groups", border_style="bright_blue"),
                Panel(table, title="Markets", border_style="bright_blue"),
            ],
            expand=True,
        ),
        Panel(footer, border_style="bright_blue"),
    )


def render_market_scan(scan: Any) -> Group:
    cards = Columns(
        [
            _metric_card("Scope", str(scan.scope).upper(), "cyan"),
            _metric_card("Scanned", str(scan.scanned_markets), "green"),
            _metric_card("Ranked", str(len(scan.rows)), "magenta", scan.timeframe),
            _metric_card("Sort", str(scan.ranked_by).upper(), "yellow", _format_short_timestamp(scan.generated_at)),
        ],
        expand=True,
        equal=True,
    )

    table = Table(box=box.SIMPLE_HEAVY, expand=True)
    table.add_column("Symbol", style="cyan", no_wrap=True)
    table.add_column("Bias", no_wrap=True)
    table.add_column("Setup")
    table.add_column("Score", justify="right")
    table.add_column("Change", justify="right")
    table.add_column("RSI", justify="right")
    table.add_column("Vol", justify="right")
    table.add_column("Group", no_wrap=True)

    for row in getattr(scan, "rows", []):
        bias_style = _market_bias_style(row.bias)
        table.add_row(
            row.symbol,
            f"[{bias_style}]{str(row.bias).upper()}[/{bias_style}]",
            row.setup,
            f"{row.score:.1f}",
            _format_signed_pct(row.price_change_pct),
            f"{row.rsi:.1f}",
            f"{row.volatility_pct:.2f}%",
            row.dex or row.scope,
        )

    if not getattr(scan, "rows", []):
        table.add_row("-", "-", "-", "-", "-", "-", "-", "-")

    summary = Text()
    summary.append(
        f"Successful scans: {scan.successful_scans}/{scan.scanned_markets}",
        style="bold green" if scan.successful_scans else "bold yellow",
    )
    if getattr(scan, "failed_markets", {}):
        summary.append(f"\nSkipped {len(scan.failed_markets)} markets", style="yellow")
        preview = list(scan.failed_markets.items())[:3]
        for symbol, reason in preview:
            summary.append(f"\n{symbol}: {reason}", style="dim")
        if len(scan.failed_markets) > len(preview):
            summary.append("\nMore failures omitted from table view; use --output json.", style="dim")

    return Group(
        cards,
        Columns(
            [
                Panel(table, title="TA Scan", border_style="bright_blue"),
                Panel(summary, title="Scan Notes", border_style="bright_blue", padding=(1, 1)),
            ],
            expand=True,
        ),
    )


def render_l2_book(
    update: Any,
    depth: int = 12,
    network: str = "mainnet",
    display_symbol: Optional[str] = None,
    subscribed_symbol: Optional[str] = None,
) -> Group:
    market_label = display_symbol or update.coin
    cards = Columns(
        [
            _metric_card("Market", market_label, "cyan", network.upper()),
            _metric_card("Mid", _format_price(update.mid_price), "green"),
            _metric_card("Spread", f"{update.spread_bps:.2f} bps", "magenta", _format_price(update.spread)),
            _metric_card("Imbalance", f"{update.imbalance:+.2f}", _signed_style(update.imbalance), _format_epoch_ms(update.timestamp_ms)),
        ],
        expand=True,
        equal=True,
    )

    asks_table = Table(box=box.SIMPLE_HEAVY, expand=True)
    asks_table.add_column("Ask Px", style="red", justify="right")
    asks_table.add_column("Size", justify="right")
    asks_table.add_column("Orders", justify="right")
    for level in getattr(update, "asks", [])[:depth]:
        asks_table.add_row(_format_price(level.price), f"{level.size:,.4f}", str(level.num_orders))
    if not getattr(update, "asks", []):
        asks_table.add_row("-", "-", "-")

    bids_table = Table(box=box.SIMPLE_HEAVY, expand=True)
    bids_table.add_column("Bid Px", style="green", justify="right")
    bids_table.add_column("Size", justify="right")
    bids_table.add_column("Orders", justify="right")
    for level in getattr(update, "bids", [])[:depth]:
        bids_table.add_row(_format_price(level.price), f"{level.size:,.4f}", str(level.num_orders))
    if not getattr(update, "bids", []):
        bids_table.add_row("-", "-", "-")

    summary_rows = [
        ("Best Bid", _format_price(update.best_bid)),
        ("Best Ask", _format_price(update.best_ask)),
        ("Total Bid Size", f"{update.total_bid_size:,.4f}"),
        ("Total Ask Size", f"{update.total_ask_size:,.4f}"),
        ("Timestamp", _format_epoch_ms(update.timestamp_ms)),
    ]

    footer = Text("Live L2 order book. Press Ctrl+C to stop.", style="dim")
    if subscribed_symbol and subscribed_symbol != market_label:
        footer.append(f"\nSubscribed as {subscribed_symbol} to match Hyperliquid data routing.", style="yellow")

    return Group(
        cards,
        Columns(
            [
                Panel(asks_table, title="Asks", border_style="red"),
                Panel(bids_table, title="Bids", border_style="green"),
                _kv_table("Book Summary", summary_rows),
            ],
            expand=True,
        ),
        Panel(footer, border_style="bright_blue"),
    )


def render_backtest_result(strategy_name: str, result: Any) -> Group:
    metrics = result.metrics
    initial_equity = result.equity_curve[0][1] if getattr(result, "equity_curve", None) else 0.0
    final_equity = result.equity_curve[-1][1] if getattr(result, "equity_curve", None) else initial_equity
    pnl = final_equity - initial_equity

    cards = Columns(
        [
            _metric_card(
                "Total Return",
                _format_signed_pct(metrics.total_return * 100.0),
                _signed_style(metrics.total_return),
                strategy_name,
            ),
            _metric_card("Final Equity", _format_currency(final_equity), "cyan", f"PnL {_format_currency(pnl)}"),
            _metric_card("Sharpe Ratio", f"{metrics.sharpe_ratio:.2f}", "magenta", f"Sortino {metrics.sortino_ratio:.2f}"),
            _metric_card("Max Drawdown", f"{metrics.max_drawdown * 100.0:.2f}%", "red", f"{metrics.total_trades} fills"),
        ],
        expand=True,
        equal=True,
    )

    performance_rows = [
        ("Period", f"{metrics.start_time.date()} to {metrics.end_time.date()}"),
        ("Trading Days", str(metrics.trading_days)),
        ("Annualized Return", f"{metrics.annualized_return * 100.0:.2f}%"),
        ("Volatility", f"{metrics.volatility * 100.0:.2f}%"),
        ("Calmar", f"{metrics.calmar_ratio:.2f}"),
        ("Avg Trade PnL", _format_currency(metrics.avg_trade_pnl)),
    ]
    trade_rows = [
        ("Trades", str(metrics.total_trades)),
        ("Winning Trades", str(metrics.winning_trades)),
        ("Losing Trades", str(metrics.losing_trades)),
        ("Win Rate", f"{metrics.win_rate * 100.0:.2f}%"),
        ("Profit Factor", f"{metrics.profit_factor:.2f}"),
        ("Commission", _format_currency(metrics.total_commission)),
    ]

    summary = Text()
    summary.append("Run summary\n", style="bold")
    if metrics.total_return > 0:
        summary.append("Profitable run", style="bold green")
    elif metrics.total_return < 0:
        summary.append("Losing run", style="bold red")
    else:
        summary.append("Flat run", style="bold yellow")
    summary.append(" with ", style="dim")
    summary.append(f"{metrics.total_trades}", style="bold cyan")
    summary.append(" trades over ", style="dim")
    summary.append(f"{metrics.trading_days}", style="bold cyan")
    summary.append(" days.\n", style="dim")
    summary.append(f"Final equity {_format_currency(final_equity)}", style="cyan")
    summary.append(" | ", style="dim")
    summary.append(f"Drawdown {metrics.max_drawdown * 100.0:.2f}%", style="red")
    summary.append(" | ", style="dim")
    summary.append(f"Sharpe {metrics.sharpe_ratio:.2f}", style="magenta")

    return Group(
        cards,
        Columns(
            [
                _kv_table("Performance", performance_rows),
                _kv_table("Trade Stats", trade_rows),
                Panel(summary, title="Overview", border_style="bright_blue", padding=(1, 1)),
            ],
            expand=True,
        ),
        _fills_table(getattr(result, "fills", []), "Recent Fills"),
    )


def render_runtime_result(result: Any, mode: str = "once") -> Group:
    duration = _format_duration(result.started_at, result.finished_at)
    orders_count = len(result.generated_orders)
    cards = Columns(
        [
            _metric_card("Strategy", result.strategy, "cyan", mode.upper()),
            _metric_card("Last Price", _format_price(result.last_price), "green", result.symbol),
            _metric_card("Candles", str(result.candles_processed), "magenta", result.timeframe),
            _metric_card("Orders", str(orders_count), "yellow", f"Duration {duration}"),
        ],
        expand=True,
        equal=True,
    )

    run_rows = [
        ("Symbol", result.symbol),
        ("Timeframe", result.timeframe),
        ("Started", _format_timestamp(result.started_at)),
        ("Finished", _format_timestamp(result.finished_at)),
        ("Duration", duration),
    ]

    if orders_count == 0:
        summary_body = Text()
        summary_body.append("No order requests were generated during this run.", style="yellow")
        summary_body.append("\nThis is useful when validating strategy wiring and market conditions.", style="dim")
        orders_panel = Panel(summary_body, title="Orders", border_style="yellow", padding=(1, 1))
    else:
        orders = []
        for order in result.generated_orders:
            enriched = dict(order)
            enriched["timestamp"] = _format_short_timestamp(result.finished_at)
            orders.append(enriched)
        orders_panel = _orders_table(orders, "Generated Orders")

    return Group(
        cards,
        Columns(
            [
                _kv_table("Run Details", run_rows),
                Panel(
                    Text(
                        f"Processed {result.candles_processed} candles and generated {orders_count} order requests.",
                        style="dim",
                    ),
                    title="Summary",
                    border_style="bright_blue",
                    padding=(1, 1),
                ),
            ],
            expand=True,
        ),
        orders_panel,
    )


def render_strategy_list(strategies: List[Dict[str, Any]]) -> Group:
    cards = Columns(
        [
            _metric_card("Strategies", str(len(strategies)), "cyan"),
            _metric_card(
                "Enabled",
                str(sum(1 for strategy in strategies if strategy.get("enabled", True))),
                "green",
            ),
        ],
        expand=True,
        equal=True,
    )

    table = Table(box=box.SIMPLE_HEAVY, expand=True)
    table.add_column("Name", style="cyan", no_wrap=True)
    table.add_column("Class", no_wrap=True)
    table.add_column("Enabled", no_wrap=True)
    table.add_column("Config", no_wrap=True)
    table.add_column("Path")

    for strategy in strategies:
        config_name = Path(strategy["config_file"]).name if strategy.get("config_file") else "-"
        table.add_row(
            str(strategy.get("name", "-")),
            str(strategy.get("class", "-")),
            "yes" if strategy.get("enabled", True) else "no",
            config_name,
            str(strategy.get("file", "-")),
        )

    return Group(cards, Panel(table, title="Project Strategies", border_style="bright_blue"))


def render_project_info(project_root: Path, config: Dict[str, Any], strategies: List[Dict[str, Any]], config_file: str) -> Group:
    tree = Tree(f"[bold cyan]{project_root.name}[/bold cyan]")
    tree.add(config_file)
    strategies_branch = tree.add("strategies/")
    for strategy in strategies:
        strategies_branch.add(Path(strategy["file"]).name)

    # Resolve database config from either [database] or legacy [storage]
    db_raw = config.get("database", config.get("storage", {}))
    db_backend = str(db_raw.get("backend", "none"))
    db_dsn = str(db_raw.get("dsn", ""))
    db_pool = str(db_raw.get("pool_size", 4))
    db_monitor = bool(db_raw.get("trade_monitoring", False))

    # Mask password in DSN for display
    import re
    db_dsn_display = re.sub(r"(://[^:]+:)[^@]+(@)", r"\1***\2", db_dsn) if db_dsn else "(not set — use NELEUS_DB_DSN)"

    db_color = {"none": "dim", "postgres": "magenta", "timescale": "cyan"}.get(db_backend, "yellow")

    cards = Columns(
        [
            _metric_card("Project", str(config.get("project", {}).get("name", project_root.name)), "cyan"),
            _metric_card("Symbol", str(config.get("market", {}).get("symbol", "BTC-PERP")), "green"),
            _metric_card("Timeframe", str(config.get("market", {}).get("timeframe", "1h")), "magenta"),
            _metric_card("DB Adapter", db_backend.upper(), db_color, "monitor=on" if db_monitor else "monitor=off"),
        ],
        expand=True,
        equal=True,
    )

    config_rows = [
        ("Project", str(config.get("project", {}).get("name", project_root.name))),
        ("Version", str(config.get("project", {}).get("version", "0.1.0"))),
        ("Symbol", str(config.get("market", {}).get("symbol", "BTC-PERP"))),
        ("Timeframe", str(config.get("market", {}).get("timeframe", "1h"))),
        ("Lookback Bars", str(config.get("market", {}).get("lookback_bars", 200))),
        ("Runtime Mode", str(config.get("runtime", {}).get("mode", "once"))),
        ("Poll Interval", f'{config.get("runtime", {}).get("poll_interval_seconds", 60)}s'),
    ]

    db_rows = [
        ("Backend", db_backend),
        ("DSN", db_dsn_display),
        ("Pool Size", db_pool),
        ("Trade Monitoring", "enabled" if db_monitor else "disabled"),
    ]

    return Group(
        cards,
        Columns(
            [
                Panel(tree, title="Project Tree", border_style="bright_blue"),
                _kv_table("Configuration", config_rows),
                _kv_table("Database Adapter", db_rows),
            ],
            expand=True,
        ),
    )


@dataclass
class DaemonDashboard:
    strategy: str
    symbol: str
    timeframe: str
    poll_interval_seconds: int
    network: str
    started_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    iteration_count: int = 0
    total_orders: int = 0
    total_candles: int = 0
    last_result: Optional[Any] = None
    recent_results: Deque[Any] = field(default_factory=lambda: deque(maxlen=12))
    recent_orders: Deque[Dict[str, Any]] = field(default_factory=lambda: deque(maxlen=12))

    def update(self, result: Any) -> None:
        self.last_result = result
        self.iteration_count += 1
        self.total_orders += len(result.generated_orders)
        self.total_candles += result.candles_processed
        self.recent_results.append(result)
        for order in result.generated_orders:
            enriched = dict(order)
            enriched["timestamp"] = _format_short_timestamp(result.finished_at)
            self.recent_orders.append(enriched)

    def _uptime(self) -> str:
        now = datetime.now(timezone.utc)
        total_seconds = max(int((now - self.started_at).total_seconds()), 0)
        hours, remainder = divmod(total_seconds, 3600)
        minutes, seconds = divmod(remainder, 60)
        if hours:
            return f"{hours}h {minutes}m {seconds}s"
        if minutes:
            return f"{minutes}m {seconds}s"
        return f"{seconds}s"

    def render(self) -> Layout:
        last_price = self.last_result.last_price if self.last_result is not None else 0.0
        last_update = _format_timestamp(self.last_result.finished_at) if self.last_result is not None else "-"
        last_orders = len(self.last_result.generated_orders) if self.last_result is not None else 0

        header_cards = Columns(
            [
                _metric_card("Strategy", self.strategy, "cyan", "DAEMON"),
                _metric_card("Symbol", self.symbol, "green", self.timeframe),
                _metric_card("Last Price", _format_price(last_price), "magenta", self.network.upper()),
                _metric_card("Orders", str(self.total_orders), "yellow", f"{self.iteration_count} updates"),
            ],
            expand=True,
            equal=True,
        )

        summary_rows = [
            ("Network", self.network),
            ("Poll Interval", f"{self.poll_interval_seconds}s"),
            ("Uptime", self._uptime()),
            ("Last Update", last_update),
            ("Candles Seen", str(self.total_candles)),
            ("Orders Last Tick", str(last_orders)),
        ]

        recent_updates = Table(box=box.SIMPLE_HEAVY, expand=True)
        recent_updates.add_column("Time", style="dim", no_wrap=True)
        recent_updates.add_column("Price", justify="right")
        recent_updates.add_column("Candles", justify="right")
        recent_updates.add_column("Orders", justify="right")

        for result in reversed(self.recent_results):
            recent_updates.add_row(
                _format_short_timestamp(result.finished_at),
                _format_price(result.last_price),
                str(result.candles_processed),
                str(len(result.generated_orders)),
            )

        if not self.recent_results:
            recent_updates.add_row("-", "-", "-", "-")

        orders = list(self.recent_orders)
        orders_panel = _orders_table(orders, "Recent Orders")

        layout = Layout()
        layout.split_column(
            Layout(header_cards, name="header", size=7),
            Layout(name="body"),
            Layout(
                Panel(
                    Text("Daemon monitor is live. Press Ctrl+C to stop.", style="dim"),
                    border_style="bright_blue",
                ),
                name="footer",
                size=3,
            ),
        )
        layout["body"].split_row(
            Layout(
                Group(
                    _kv_table("Runtime", summary_rows),
                    Panel(recent_updates, title="Recent Updates", border_style="bright_blue"),
                ),
                name="left",
                ratio=2,
            ),
            Layout(orders_panel, name="right", ratio=3),
        )
        return layout
