"""
Hyperliquid market analysis helpers.
"""

from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Optional, Tuple

import numpy as np

from .constants import (
    BOLLINGER_STD,
    EMA_FAST,
    EMA_SLOW,
    RSI_OVERBOUGHT,
    RSI_OVERSOLD,
    RSI_PERIOD,
    SMA_PERIOD,
)
from .types import HyperliquidClient

DEFAULT_SCAN_SYMBOLS = (
    "BTC",
    "ETH",
    "SOL",
    "HYPE",
    "XRP",
    "DOGE",
    "SUI",
    "AAVE",
)

SCOPE_ALIASES = {
    "perp": "perps",
    "perps": "perps",
    "perpetual": "perps",
    "perpetuals": "perps",
    "hip3": "hip3",
    "hip-3": "hip3",
    "spot": "spot",
    "spots": "spot",
    "all-perp": "all-perps",
    "all-perps": "all-perps",
    "all_perps": "all-perps",
    "hip4": "hip4",
    "hip-4": "hip4",
    "outcome": "hip4",
    "outcomes": "hip4",
}


TIMEFRAME_TO_MINUTES = {
    "1m": 1,
    "5m": 5,
    "15m": 15,
    "1h": 60,
    "4h": 240,
    "1d": 1440,
}


def normalize_market_symbol(symbol: str) -> str:
    cleaned = symbol.strip()
    return cleaned[:-5] if cleaned.upper().endswith("-PERP") else cleaned


def _symbol_aliases(name: str) -> List[str]:
    normalized = normalize_market_symbol(name)
    aliases = {normalized, normalized.upper(), normalized.lower()}
    if ":" in normalized:
        _, suffix = normalized.split(":", 1)
        aliases.update({suffix, suffix.upper(), suffix.lower()})
    return [alias for alias in aliases if alias]


def _split_market_symbol(symbol: str) -> Tuple[Optional[str], str]:
    normalized = normalize_market_symbol(symbol)
    if ":" not in normalized:
        return None, normalized
    return tuple(normalized.split(":", 1))  # type: ignore[return-value]


def _timeframe_to_millis(timeframe: str) -> int:
    minutes = TIMEFRAME_TO_MINUTES.get(timeframe)
    if minutes is None:
        raise ValueError(f"Unsupported timeframe '{timeframe}'")
    return minutes * 60 * 1000


def normalize_market_scope(scope: str) -> str:
    normalized = scope.strip().lower().replace("_", "-")
    canonical = SCOPE_ALIASES.get(normalized)
    if canonical is None:
        raise ValueError("scope must be one of: perps, hip3, spot, all-perps, hip4")
    return canonical


def _ema(values: np.ndarray, period: int) -> float:
    if len(values) < period:
        return float(values[-1])

    alpha = 2.0 / (period + 1)
    ema_value = float(values[0])
    for value in values[1:]:
        ema_value = alpha * float(value) + (1.0 - alpha) * ema_value
    return ema_value


def fetch_market_candles(
    symbol: str,
    timeframe: str = "1h",
    lookback_bars: int = 200,
    testnet: bool = False,
    dex: Optional[str] = None,
) -> List[Dict[str, Any]]:
    client = HyperliquidClient(testnet=testnet)
    end_time = datetime.now(timezone.utc)
    lookback = timedelta(milliseconds=_timeframe_to_millis(timeframe) * max(lookback_bars, 2))
    start_time = end_time - lookback
    normalized = normalize_market_symbol(symbol)
    inferred_dex, base_symbol = _split_market_symbol(normalized)
    resolved_dex = dex or inferred_dex
    candidates: List[Tuple[str, Optional[str]]] = []

    if resolved_dex:
        if ":" in normalized:
            candidates.append((normalized, None))
        candidates.append((base_symbol, resolved_dex))
        if base_symbol != normalized:
            candidates.append((base_symbol, None))
    else:
        candidates.append((normalized, None))
        if base_symbol != normalized:
            candidates.append((base_symbol, None))

    seen: set[Tuple[str, Optional[str]]] = set()
    deduped_candidates: List[Tuple[str, Optional[str]]] = []
    for candidate in candidates:
        if candidate in seen:
            continue
        seen.add(candidate)
        deduped_candidates.append(candidate)

    candles = []
    for request_symbol, request_dex in deduped_candidates:
        candles = client.fetch_candles(
            request_symbol,
            timeframe,
            int(start_time.timestamp() * 1000),
            int(end_time.timestamp() * 1000),
            request_dex,
        )
        if candles:
            break

    return [
        {
            "timestamp": int(candle.timestamp),
            "open": float(candle.open),
            "high": float(candle.high),
            "low": float(candle.low),
            "close": float(candle.close),
            "volume": float(candle.volume),
            "num_trades": int(candle.num_trades),
        }
        for candle in candles
    ]


@dataclass
class MarketEntry:
    name: str
    scope: str
    market_type: str
    dex: Optional[str] = None
    collateral_token: Optional[str] = None
    max_leverage: Optional[int] = None
    sz_decimals: Optional[int] = None
    base_token: Optional[str] = None
    quote_token: Optional[str] = None
    token_indices: List[int] = field(default_factory=list)
    is_canonical: Optional[bool] = None
    full_name: Optional[str] = None
    description: Optional[str] = None
    request_symbol: Optional[str] = None
    request_dex: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "scope": self.scope,
            "market_type": self.market_type,
            "dex": self.dex,
            "collateral_token": self.collateral_token,
            "max_leverage": self.max_leverage,
            "sz_decimals": self.sz_decimals,
            "base_token": self.base_token,
            "quote_token": self.quote_token,
            "token_indices": self.token_indices,
            "is_canonical": self.is_canonical,
            "full_name": self.full_name,
            "description": self.description,
            "request_symbol": self.request_symbol,
            "request_dex": self.request_dex,
        }


@dataclass
class MarketCatalog:
    scope: str
    total_markets: int
    entries: List[MarketEntry]
    dex_counts: Dict[str, int]
    total_tokens: int = 0
    generated_at: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "scope": self.scope,
            "total_markets": self.total_markets,
            "dex_counts": self.dex_counts,
            "total_tokens": self.total_tokens,
            "generated_at": self.generated_at,
            "entries": [entry.to_dict() for entry in self.entries],
        }


def _market_dex(name: str) -> Optional[str]:
    if ":" not in name:
        return None
    return name.split(":", 1)[0]


def _search_rank(entry: MarketEntry, search: Optional[str]) -> Optional[int]:
    if not search:
        return 0

    needle = normalize_market_symbol(search).lower()
    normalized_name = normalize_market_symbol(entry.name).lower()
    suffix = entry.name.split(":", 1)[1].lower() if ":" in entry.name else normalized_name
    aliases = {alias.lower() for alias in _symbol_aliases(entry.name)}

    if needle in aliases:
        return 0
    if suffix.startswith(needle) or normalized_name.startswith(needle):
        return 1
    if needle in suffix or needle in normalized_name or needle in entry.name.lower():
        return 2
    if entry.base_token and needle in entry.base_token.lower():
        return 3
    if entry.quote_token and needle in entry.quote_token.lower():
        return 3
    if entry.full_name and needle in entry.full_name.lower():
        return 4
    if entry.dex and needle in entry.dex.lower():
        return 5
    if needle in entry.scope.lower() or needle in entry.market_type.lower():
        return 6
    return None


def _book_request_candidates(symbol: str) -> List[str]:
    normalized = normalize_market_symbol(symbol)
    _, base_symbol = _split_market_symbol(normalized)
    candidates = [normalized]
    if base_symbol != normalized:
        candidates.append(base_symbol)
    return list(dict.fromkeys(candidate for candidate in candidates if candidate))


def _has_book_liquidity(update: Any) -> bool:
    return bool(
        update is not None
        and (getattr(update, "bids", None) or getattr(update, "asks", None))
        and (getattr(update, "best_bid", 0.0) > 0.0 or getattr(update, "best_ask", 0.0) > 0.0)
    )


def resolve_market_entry(
    symbol: str,
    scope: str = "all-perps",
    dex: Optional[str] = None,
    testnet: bool = False,
) -> MarketEntry:
    query = normalize_market_symbol(symbol)
    catalog = list_markets(scope=scope, dex=dex, search=None, testnet=testnet)

    if not catalog.entries:
        raise ValueError(f"No markets available for scope={scope!r}")

    exact_matches: List[MarketEntry] = []
    partial_matches: List[MarketEntry] = []
    query_lower = query.lower()

    for entry in catalog.entries:
        aliases = _symbol_aliases(entry.name)
        if query in aliases or query.upper() in aliases or query_lower in aliases:
            exact_matches.append(entry)
            continue

        haystack = " ".join(
            [
                entry.name,
                normalize_market_symbol(entry.name),
                entry.dex or "",
                entry.base_token or "",
                entry.quote_token or "",
                entry.full_name or "",
            ]
        ).lower()
        if query_lower in haystack:
            partial_matches.append(entry)

    if len(exact_matches) == 1:
        return exact_matches[0]

    if len(exact_matches) > 1:
        ranked = sorted(exact_matches, key=lambda entry: (entry.dex or "", entry.name))
        suggestions = ", ".join(entry.name for entry in ranked[:6])
        raise ValueError(f"Ambiguous market {symbol!r}. Try one of: {suggestions}")

    if len(partial_matches) == 1:
        return partial_matches[0]

    if partial_matches:
        ranked = sorted(partial_matches, key=lambda entry: (entry.dex or "", entry.name))
        suggestions = ", ".join(entry.name for entry in ranked[:6])
        raise ValueError(f"Multiple markets match {symbol!r}. Try one of: {suggestions}")

    raise ValueError(f"No Hyperliquid market matched {symbol!r} in scope={scope!r}")


def resolve_market_symbol(
    symbol: str,
    scope: str = "all-perps",
    dex: Optional[str] = None,
    testnet: bool = False,
) -> str:
    return resolve_market_entry(symbol, scope=scope, dex=dex, testnet=testnet).name


def _spot_token_names(client: HyperliquidClient) -> Dict[str, str]:
    spot_meta = client.fetch_spot_meta()
    return {str(token.index): token.name for token in spot_meta.tokens}


def _resolve_collateral_token(
    raw_value: Optional[str], token_names: Optional[Dict[str, str]]
) -> Optional[str]:
    if raw_value is None:
        return None

    value = str(raw_value)
    if not token_names:
        return value

    return token_names.get(value, value)


def list_markets(
    scope: str = "perps",
    dex: Optional[str] = None,
    search: Optional[str] = None,
    testnet: bool = False,
) -> MarketCatalog:
    client = HyperliquidClient(testnet=testnet)
    normalized_scope = normalize_market_scope(scope)
    entries: List[MarketEntry] = []
    dex_counts: Dict[str, int] = {}
    total_tokens = 0
    collateral_token_names: Optional[Dict[str, str]] = None

    if normalized_scope in {"perps", "all-perps", "hip3"}:
        collateral_token_names = _spot_token_names(client)

    if normalized_scope == "perps":
        meta = client.fetch_meta()
        entries = [
            MarketEntry(
                name=asset.name,
                scope="perps",
                market_type="perp",
                dex="default",
                collateral_token=_resolve_collateral_token(
                    getattr(meta, "collateral_token", None), collateral_token_names
                ),
                max_leverage=asset.max_leverage,
                sz_decimals=asset.sz_decimals,
                request_symbol=asset.name,
            )
            for asset in meta.symbols
        ]
        dex_counts = {"default": len(entries)}
    elif normalized_scope == "all-perps":
        metas = client.fetch_all_perp_metas()
        for meta in metas:
            collateral_token = _resolve_collateral_token(
                getattr(meta, "collateral_token", None), collateral_token_names
            )
            for asset in meta.symbols:
                market_dex = _market_dex(asset.name) or "default"
                entries.append(
                    MarketEntry(
                        name=asset.name,
                        scope="all-perps",
                        market_type="perp",
                        dex=market_dex,
                        collateral_token=collateral_token,
                        max_leverage=asset.max_leverage,
                        sz_decimals=asset.sz_decimals,
                        request_symbol=asset.name.split(":", 1)[1] if market_dex != "default" and ":" in asset.name else asset.name,
                        request_dex=market_dex if market_dex != "default" else None,
                    )
                )
                dex_counts[market_dex] = dex_counts.get(market_dex, 0) + 1
    elif normalized_scope == "hip3":
        if dex:
            meta = client.fetch_meta(dex)
            collateral_token = _resolve_collateral_token(
                getattr(meta, "collateral_token", None), collateral_token_names
            )
            entries = [
                MarketEntry(
                    name=asset.name,
                    scope="hip3",
                    market_type="perp",
                    dex=_market_dex(asset.name) or dex,
                    collateral_token=collateral_token,
                    max_leverage=asset.max_leverage,
                    sz_decimals=asset.sz_decimals,
                    request_symbol=asset.name.split(":", 1)[1] if ":" in asset.name else asset.name,
                    request_dex=dex,
                )
                for asset in meta.symbols
            ]
            dex_counts = {dex: len(entries)}
        else:
            metas = client.fetch_all_perp_metas()
            for meta in metas:
                collateral_token = _resolve_collateral_token(
                    getattr(meta, "collateral_token", None), collateral_token_names
                )
                for asset in meta.symbols:
                    market_dex = _market_dex(asset.name)
                    if market_dex is None:
                        continue
                    entries.append(
                        MarketEntry(
                            name=asset.name,
                            scope="hip3",
                            market_type="perp",
                            dex=market_dex,
                            collateral_token=collateral_token,
                            max_leverage=asset.max_leverage,
                            sz_decimals=asset.sz_decimals,
                            request_symbol=asset.name.split(":", 1)[1] if ":" in asset.name else asset.name,
                            request_dex=market_dex,
                        )
                    )
                    dex_counts[market_dex] = dex_counts.get(market_dex, 0) + 1
    elif normalized_scope == "spot":
        meta = client.fetch_spot_meta()
        token_by_index = {token.index: token for token in meta.tokens}
        total_tokens = len(meta.tokens)
        for market in meta.markets:
            base_token = (
                token_by_index[market.tokens[0]].name
                if len(market.tokens) > 0 and market.tokens[0] in token_by_index
                else None
            )
            quote_token = (
                token_by_index[market.tokens[1]].name
                if len(market.tokens) > 1 and market.tokens[1] in token_by_index
                else None
            )
            entries.append(
                MarketEntry(
                    name=market.name,
                    scope="spot",
                    market_type="spot",
                    dex="spot",
                    base_token=base_token,
                    quote_token=quote_token,
                    token_indices=list(market.tokens),
                    is_canonical=market.is_canonical,
                    request_symbol=market.name,
                )
            )
        dex_counts = {"spot": len(entries)}
    elif normalized_scope == "hip4":
        if not testnet:
            raise ValueError(
                "HIP-4 outcome markets are testnet-only. Pass testnet=True (or --testnet on the CLI)."
            )
        meta = client.fetch_outcome_meta()
        for outcome in meta.outcomes:
            if len(outcome.side_specs) > 2:
                raise ValueError(
                    f"HIP-4 outcome '{outcome.name}' has {len(outcome.side_specs)} side specs; expected at most 2."
                )
            for side, spec in enumerate(outcome.side_specs[:2]):
                encoding = 10 * outcome.outcome + side
                coin = f"#{encoding}"
                entries.append(
                    MarketEntry(
                        name=coin,
                        scope="hip4",
                        market_type="outcome",
                        dex="outcome",
                        full_name=f"{outcome.name} — {spec.name}",
                        description=outcome.description,
                        request_symbol=coin,
                    )
                )
        dex_counts = {"outcome": len(entries)}
    else:
        raise ValueError("scope must be one of: perps, hip3, spot, all-perps, hip4")

    ranked_entries = []
    for entry in entries:
        rank = _search_rank(entry, search)
        if rank is None:
            continue
        ranked_entries.append((rank, entry))
    ranked_entries.sort(key=lambda item: (item[0], item[1].dex or "", item[1].name))
    filtered = [entry for _, entry in ranked_entries]

    filtered_counts: Dict[str, int] = {}
    for entry in filtered:
        key = entry.dex or entry.market_type
        filtered_counts[key] = filtered_counts.get(key, 0) + 1

    return MarketCatalog(
        scope=normalized_scope,
        total_markets=len(filtered),
        entries=filtered,
        dex_counts=filtered_counts if filtered else dex_counts,
        total_tokens=total_tokens,
        generated_at=datetime.now(timezone.utc).isoformat(),
    )


@dataclass
class MarketAnalysis:
    symbol: str
    timeframe: str
    last_price: float
    price_change_pct: float
    volatility_pct: float
    rsi: float
    sma_fast: float
    sma_slow: float
    ema_fast: float
    ema_slow: float
    bollinger_upper: float
    bollinger_mid: float
    bollinger_lower: float
    support: float
    resistance: float
    trend: str
    momentum: str
    bias: str
    candles_analyzed: int
    generated_at: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "symbol": self.symbol,
            "timeframe": self.timeframe,
            "last_price": self.last_price,
            "price_change_pct": self.price_change_pct,
            "volatility_pct": self.volatility_pct,
            "rsi": self.rsi,
            "sma_fast": self.sma_fast,
            "sma_slow": self.sma_slow,
            "ema_fast": self.ema_fast,
            "ema_slow": self.ema_slow,
            "bollinger_upper": self.bollinger_upper,
            "bollinger_mid": self.bollinger_mid,
            "bollinger_lower": self.bollinger_lower,
            "support": self.support,
            "resistance": self.resistance,
            "trend": self.trend,
            "momentum": self.momentum,
            "bias": self.bias,
            "candles_analyzed": self.candles_analyzed,
            "generated_at": self.generated_at,
        }


@dataclass
class MarketScanRow:
    symbol: str
    scope: str
    dex: Optional[str]
    last_price: float
    price_change_pct: float
    volatility_pct: float
    rsi: float
    trend: str
    momentum: str
    bias: str
    setup: str
    score: float

    def to_dict(self) -> Dict[str, Any]:
        return {
            "symbol": self.symbol,
            "scope": self.scope,
            "dex": self.dex,
            "last_price": self.last_price,
            "price_change_pct": self.price_change_pct,
            "volatility_pct": self.volatility_pct,
            "rsi": self.rsi,
            "trend": self.trend,
            "momentum": self.momentum,
            "bias": self.bias,
            "setup": self.setup,
            "score": self.score,
        }


@dataclass
class MarketScan:
    scope: str
    timeframe: str
    lookback_bars: int
    scanned_markets: int
    successful_scans: int
    ranked_by: str
    rows: List[MarketScanRow]
    failed_markets: Dict[str, str]
    generated_at: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "scope": self.scope,
            "timeframe": self.timeframe,
            "lookback_bars": self.lookback_bars,
            "scanned_markets": self.scanned_markets,
            "successful_scans": self.successful_scans,
            "ranked_by": self.ranked_by,
            "generated_at": self.generated_at,
            "failed_markets": self.failed_markets,
            "rows": [row.to_dict() for row in self.rows],
        }


def _analysis_setup(analysis: MarketAnalysis) -> str:
    if analysis.bias == "long" and analysis.momentum == "oversold":
        return "pullback long"
    if analysis.bias == "long" and analysis.momentum == "rising":
        return "trend continuation"
    if analysis.bias == "short" and analysis.momentum == "overbought":
        return "fade rally"
    if analysis.bias == "short" and analysis.momentum == "fading":
        return "trend breakdown"
    if analysis.trend == "sideways":
        return "range watch"
    return "watch"


def _analysis_score(analysis: MarketAnalysis) -> float:
    score = 18.0
    score += {"long": 18.0, "short": 18.0, "neutral": 5.0}.get(analysis.bias, 5.0)
    score += {"bullish": 14.0, "bearish": 14.0, "sideways": 4.0}.get(analysis.trend, 4.0)
    score += {
        "rising": 12.0,
        "fading": 12.0,
        "oversold": 10.0,
        "overbought": 10.0,
    }.get(analysis.momentum, 6.0)
    score += max(0.0, 12.0 - (abs(analysis.rsi - 50.0) / 2.5))
    score += min(abs(analysis.price_change_pct) * 0.8, 16.0)
    score -= min(analysis.volatility_pct * 2.5, 18.0)
    return round(max(0.0, min(score, 100.0)), 2)


def _prioritize_scan_entries(entries: List[MarketEntry]) -> List[MarketEntry]:
    preferred_order = {
        symbol: index
        for index, symbol in enumerate(DEFAULT_SCAN_SYMBOLS)
    }
    return sorted(
        entries,
        key=lambda entry: (
            preferred_order.get(normalize_market_symbol(entry.name).upper(), 10_000),
            entry.dex or "",
            entry.name,
        ),
    )


def analyze_market(
    symbol: str,
    timeframe: str = "1h",
    lookback_bars: int = 200,
    testnet: bool = False,
    dex: Optional[str] = None,
    display_symbol: Optional[str] = None,
) -> MarketAnalysis:
    candles = fetch_market_candles(
        symbol=symbol,
        timeframe=timeframe,
        lookback_bars=lookback_bars,
        testnet=testnet,
        dex=dex,
    )

    if len(candles) < SMA_PERIOD:
        raise ValueError(
            f"Need at least {SMA_PERIOD} candles to analyze {symbol}; got {len(candles)}"
        )

    closes = np.array([candle["close"] for candle in candles], dtype=float)
    highs = np.array([candle["high"] for candle in candles], dtype=float)
    lows = np.array([candle["low"] for candle in candles], dtype=float)

    last_price = float(closes[-1])
    price_change_pct = float(((closes[-1] / closes[0]) - 1.0) * 100.0)

    returns = np.diff(closes) / closes[:-1]
    volatility_pct = float(np.std(returns) * 100.0) if len(returns) else 0.0

    delta = np.diff(closes)
    gain = np.where(delta > 0, delta, 0.0)
    loss = np.where(delta < 0, -delta, 0.0)
    avg_gain = float(np.mean(gain[-RSI_PERIOD:])) if len(gain) >= RSI_PERIOD else 0.0
    avg_loss = float(np.mean(loss[-RSI_PERIOD:])) if len(loss) >= RSI_PERIOD else 0.0
    if avg_loss == 0.0:
        rsi = 100.0
    else:
        rs = avg_gain / avg_loss
        rsi = 100.0 - (100.0 / (1.0 + rs))

    sma_fast = float(np.mean(closes[-SMA_PERIOD:]))
    sma_slow_period = min(50, len(closes))
    sma_slow = float(np.mean(closes[-sma_slow_period:]))
    ema_fast = _ema(closes, EMA_FAST)
    ema_slow = _ema(closes, EMA_SLOW)

    rolling_window = closes[-SMA_PERIOD:]
    bollinger_mid = float(np.mean(rolling_window))
    rolling_std = float(np.std(rolling_window))
    bollinger_upper = bollinger_mid + (BOLLINGER_STD * rolling_std)
    bollinger_lower = bollinger_mid - (BOLLINGER_STD * rolling_std)

    support = float(np.min(lows[-SMA_PERIOD:]))
    resistance = float(np.max(highs[-SMA_PERIOD:]))

    trend = "bullish" if last_price >= sma_fast >= sma_slow else "bearish"
    if abs(last_price - sma_fast) / max(last_price, 1e-9) < 0.005:
        trend = "sideways"

    if rsi < RSI_OVERSOLD:
        momentum = "oversold"
    elif rsi > RSI_OVERBOUGHT:
        momentum = "overbought"
    elif ema_fast > ema_slow:
        momentum = "rising"
    else:
        momentum = "fading"

    if trend == "bullish" and momentum in {"rising", "oversold"}:
        bias = "long"
    elif trend == "bearish" and momentum in {"fading", "overbought"}:
        bias = "short"
    else:
        bias = "neutral"

    return MarketAnalysis(
        symbol=display_symbol or symbol,
        timeframe=timeframe,
        last_price=last_price,
        price_change_pct=round(price_change_pct, 4),
        volatility_pct=round(volatility_pct, 4),
        rsi=round(float(rsi), 4),
        sma_fast=round(sma_fast, 4),
        sma_slow=round(sma_slow, 4),
        ema_fast=round(ema_fast, 4),
        ema_slow=round(ema_slow, 4),
        bollinger_upper=round(bollinger_upper, 4),
        bollinger_mid=round(bollinger_mid, 4),
        bollinger_lower=round(bollinger_lower, 4),
        support=round(support, 4),
        resistance=round(resistance, 4),
        trend=trend,
        momentum=momentum,
        bias=bias,
        candles_analyzed=len(candles),
        generated_at=datetime.now(timezone.utc).isoformat(),
    )


def scan_markets(
    scope: str = "perps",
    dex: Optional[str] = None,
    search: Optional[str] = None,
    symbols: Optional[List[str]] = None,
    timeframe: str = "1h",
    lookback_bars: int = 200,
    max_markets: int = 8,
    limit: int = 8,
    sort_by: str = "score",
    testnet: bool = False,
) -> MarketScan:
    if max_markets <= 0:
        raise ValueError("max_markets must be greater than 0")
    if limit <= 0:
        raise ValueError("limit must be greater than 0")

    if symbols:
        selected_entries = []
        for symbol in symbols:
            if not symbol.strip():
                continue
            resolved = resolve_market_entry(
                symbol.strip(),
                scope=scope,
                dex=dex,
                testnet=testnet,
            )
            selected_entries.append(
                MarketEntry(
                    name=resolved.name,
                    scope="custom",
                    market_type="custom",
                    dex=resolved.dex,
                    request_symbol=resolved.request_symbol,
                    request_dex=resolved.request_dex,
                )
            )
        selected_entries = selected_entries[:max_markets]
        scan_scope = "custom"
    else:
        catalog = list_markets(scope=scope, dex=dex, search=search, testnet=testnet)
        selected_entries = _prioritize_scan_entries(catalog.entries)[:max_markets]
        scan_scope = catalog.scope

    if not selected_entries:
        raise ValueError("No markets available for scan")

    rows: List[MarketScanRow] = []
    failed_markets: Dict[str, str] = {}
    max_workers = min(6, len(selected_entries))

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {
            executor.submit(
                analyze_market,
                symbol=entry.name,
                timeframe=timeframe,
                lookback_bars=lookback_bars,
                testnet=testnet,
                dex=entry.request_dex or entry.dex,
                display_symbol=entry.name,
            ): entry
            for entry in selected_entries
        }

        for future in as_completed(futures):
            entry = futures[future]
            try:
                analysis = future.result()
            except Exception as exc:  # noqa: BLE001 - surfaced to CLI summary
                failed_markets[entry.name] = str(exc)
                continue

            rows.append(
                MarketScanRow(
                    symbol=entry.name,
                    scope=entry.scope,
                    dex=entry.dex,
                    last_price=analysis.last_price,
                    price_change_pct=analysis.price_change_pct,
                    volatility_pct=analysis.volatility_pct,
                    rsi=analysis.rsi,
                    trend=analysis.trend,
                    momentum=analysis.momentum,
                    bias=analysis.bias,
                    setup=_analysis_setup(analysis),
                    score=_analysis_score(analysis),
                )
            )

    if sort_by == "score":
        rows.sort(key=lambda row: (-row.score, row.symbol))
    elif sort_by == "change":
        rows.sort(key=lambda row: (-abs(row.price_change_pct), -row.score, row.symbol))
    elif sort_by == "volatility":
        rows.sort(key=lambda row: (-row.volatility_pct, -row.score, row.symbol))
    elif sort_by == "rsi":
        rows.sort(key=lambda row: (-abs(row.rsi - 50.0), -row.score, row.symbol))
    else:
        raise ValueError("sort_by must be one of: score, change, volatility, rsi")

    return MarketScan(
        scope=scan_scope,
        timeframe=timeframe,
        lookback_bars=lookback_bars,
        scanned_markets=len(selected_entries),
        successful_scans=len(rows),
        ranked_by=sort_by,
        rows=rows[:limit],
        failed_markets=failed_markets,
        generated_at=datetime.now(timezone.utc).isoformat(),
    )


def open_l2_book_stream(
    symbol: str,
    testnet: bool = False,
    timeout_ms: int = 4_000,
) -> Tuple[Any, Optional[Any], str]:
    client = HyperliquidClient(testnet=testnet)
    last_stream = None
    last_update = None
    snapshot_coin = normalize_market_symbol(symbol)
    last_coin = normalize_market_symbol(symbol)

    for request_symbol in _book_request_candidates(symbol):
        snapshot = client.fetch_l2_book(request_symbol)
        if _has_book_liquidity(snapshot):
            snapshot_coin = request_symbol
            last_update = snapshot

        stream = client.stream_l2_book(request_symbol)
        last_stream = stream
        last_coin = request_symbol
        update = None
        attempts = max(1, timeout_ms // 750)
        for _ in range(attempts):
            update = stream.next_update(timeout_ms=750)
            if update is not None:
                break

        if update is None:
            if _has_book_liquidity(last_update) and request_symbol == snapshot_coin:
                return stream, last_update, request_symbol
            continue

        if _has_book_liquidity(update):
            return stream, update, request_symbol

        if last_update is None and update is not None:
            last_update = update

    return last_stream, last_update, snapshot_coin if _has_book_liquidity(last_update) else last_coin


def stream_l2_book(symbol: str, testnet: bool = False):
    stream, _, _ = open_l2_book_stream(symbol, testnet=testnet)
    return stream


__all__ = [
    "MarketCatalog",
    "MarketEntry",
    "MarketAnalysis",
    "MarketScan",
    "MarketScanRow",
    "analyze_market",
    "fetch_market_candles",
    "list_markets",
    "normalize_market_symbol",
    "open_l2_book_stream",
    "resolve_market_entry",
    "resolve_market_symbol",
    "scan_markets",
    "stream_l2_book",
]
