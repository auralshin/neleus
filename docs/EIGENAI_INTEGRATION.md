# EigenAI Integration for Neleus

## Overview

EigenAI is a deterministic, verifiable LLM inference service that provides an OpenAI-compatible API for executing open source LLMs. Unlike traditional AI services where you trust the provider's outputs, EigenAI enables cryptographic verification where inference is executed using the specified model and input, and the output results are untampered.

**Key Properties:**
- **Deterministic Execution**: Same request → Same output (bit-for-bit) every time
- **OpenAI-Compatible API**: Drop-in replacement for existing integrations
- **Verifiable**: Ability to prove inference was executed correctly
- **Open Source LLMs**: Uses models like Llama-3.3-70B-Instruct

**Why This Matters for Trading:**
Traditional LLM APIs (OpenAI, Anthropic) are non-deterministic, making them unsuitable for:
- Reproducible backtesting of AI-powered strategies
- Auditing and compliance (proving what the AI decided)
- Consistent trading decisions in production

## Key Synergies with Neleus

### 1. Deterministic Backtesting for AI Strategies

**Problem**: Traditional LLM APIs are non-deterministic → Can't backtest AI strategies reliably  
**Solution**: EigenAI guarantees same input + same seed = same output

```python
# User's AI-powered strategy
class LLMSentimentStrategy(Strategy):
    def on_data(self, data):
        # Call EigenAI with market data
        analysis = eigenai_client.analyze(
            prompt=f"Analyze BTC price action: {data}",
            seed=self.deterministic_seed  # Reproducible
        )
        
        # Same input always produces same signal
        if analysis.sentiment == "bullish":
            self.buy(size=0.1)

# Backtest is now reproducible!
backtest = Backtest(strategy=LLMSentimentStrategy())
result1 = backtest.run()  # P&L: +15.3%
result2 = backtest.run()  # P&L: +15.3% (identical)
```

### 2. Verifiable Trading Decisions

**Use Cases:**
- Prove to regulators what AI "decided" at specific timestamp
- Reproduce exact trading signal for dispute resolution
- Audit trail: "This trade was made because model X with prompt Y returned Z"
- Third-party verification of AI trading decisions

### 3. Prediction Market Agents (Polymarket Integration)

Neleus already supports Polymarket! EigenAI enables:

```python
# Automated prediction market agent
class PredictionMarketAgent(Strategy):
    def monitor_events(self):
        news = self.fetch_news_feed()
        
        # EigenAI analyzes event outcome
        analysis = eigenai.classify_event(
            event="Will Bitcoin hit $150k by March 2026?",
            news_context=news,
            seed=hash(news)  # Deterministic based on news
        )
        
        # Place bet via Neleus
        if analysis.probability > 0.7:
            self.place_bet(
                market="polymarket/btc-150k",
                outcome="YES",
                size=1000
            )
```

**Value**: Can prove AI's reasoning if market settlement is disputed

### 4. Multi-Modal Strategy Intelligence

Combine traditional indicators with AI reasoning:

```
Traditional Indicators (Rust, microsecond latency):
  ✓ RSI, MACD, Bollinger Bands
  ✓ Order book imbalance
  ✓ Volume profile
  
+ EigenAI Analysis (Python, second latency):
  ✓ News sentiment
  ✓ Social media momentum  
  ✓ Cross-market correlations
  ✓ Complex pattern recognition
  ✓ Natural language event interpretation
  
→ Unified signal generation in Neleus
```

### 5. Natural Language Risk Assessment

```python
risk_prompt = f"""
Current portfolio:
- BTC: Long 2.5 @ $100k (unrealized P&L: +$5k)
- ETH: Short 10 @ $3k (unrealized P&L: -$2k)
- Max drawdown: 8%
- Sharpe: 1.2

Market conditions:
- BTC 24h volatility: 45%
- Correlation BTC-ETH: 0.85
- Funding rates: +0.05%
- Recent news: "SEC approves Bitcoin ETF options"

Assess risk level (LOW/MEDIUM/HIGH) and recommend position adjustments.
Respond in JSON format.
"""

# Deterministic risk assessment
risk = eigenai.analyze(prompt, seed=hash(portfolio_state))
if risk.level == "HIGH":
    portfolio.reduce_exposure(0.5)
```

## Implementation Options

### Option 1: AI Strategy Adapter (Minimal Integration)

Add new strategy type that calls EigenAI:

**File**: `crates/adapters-eigenai/src/lib.rs`

```rust
use serde::{Deserialize, Serialize};
use reqwest::Client;

pub struct EigenAIAdapter {
    client: Client,
    endpoint: String,
    model: String,
    api_key: String,
    seed_strategy: SeedStrategy,
}

pub enum SeedStrategy {
    Fixed(u64),              // Always same seed (for backtesting)
    Timestamp,               // Hash of market timestamp
    MarketData,              // Hash of price/volume (same conditions = same seed)
    PromptContent,           // Hash of prompt itself
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    seed: u64,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

impl EigenAIAdapter {
    pub fn new(
        endpoint: String,
        model: String,
        api_key: String,
        seed_strategy: SeedStrategy,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
            api_key,
            seed_strategy,
        }
    }
    
    pub async fn generate_signal(
        &self,
        prompt: &str,
        context_data: &[u8],  // Market data for seed generation
    ) -> Result<String, Box<dyn std::error::Error>> {
        let seed = self.compute_seed(prompt, context_data);
        
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            seed,
            temperature: 0.0,  // Deterministic
        };
        
        let response = self.client
            .post(&format!("{}/v1/chat/completions", self.endpoint))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;
        
        Ok(response.choices[0].message.content.clone())
    }
    
    fn compute_seed(&self, prompt: &str, context: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        match self.seed_strategy {
            SeedStrategy::Fixed(seed) => seed,
            SeedStrategy::Timestamp => {
                // Use nanosecond timestamp
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64
            }
            SeedStrategy::MarketData => {
                let mut hasher = DefaultHasher::new();
                context.hash(&mut hasher);
                hasher.finish()
            }
            SeedStrategy::PromptContent => {
                let mut hasher = DefaultHasher::new();
                prompt.hash(&mut hasher);
                hasher.finish()
            }
        }
    }
}
```

### Option 2: Python Bindings for AI Strategies

**File**: `python/neleus/ai.py`

```python
from typing import Optional, Dict, Any, List
import hashlib
import json
from openai import OpenAI


class EigenAIClient:
    """Client for EigenAI deterministic LLM inference."""
    
    def __init__(
        self,
        api_key: str,
        endpoint: str = "https://api.eigenai.com/v1",
        model: str = "meta-llama/Llama-3.3-70B-Instruct",
    ):
        self.client = OpenAI(
            api_key=api_key,
            base_url=endpoint,
        )
        self.model = model
    
    def chat(
        self,
        prompt: str,
        seed: Optional[int] = None,
        system_message: Optional[str] = None,
        temperature: float = 0.0,
    ) -> str:
        """
        Send a chat request to EigenAI.
        
        Args:
            prompt: User prompt
            seed: Seed for deterministic output. If None, generates from prompt hash
            system_message: Optional system message
            temperature: Temperature (0.0 for deterministic)
        
        Returns:
            Response content
        """
        messages = []
        if system_message:
            messages.append({"role": "system", "content": system_message})
        messages.append({"role": "user", "content": prompt})
        
        if seed is None:
            seed = self._hash_to_seed(prompt)
        
        response = self.client.chat.completions.create(
            model=self.model,
            messages=messages,
            seed=seed,
            temperature=temperature,
        )
        
        return response.choices[0].message.content
    
    def analyze_market(
        self,
        symbol: str,
        price: float,
        volume: float,
        indicators: Dict[str, float],
        news: Optional[List[str]] = None,
        seed: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Analyze market conditions using LLM.
        
        Returns JSON with: sentiment, confidence, action, reasoning
        """
        prompt = f"""Analyze the following market data and provide trading signal:

Symbol: {symbol}
Current Price: ${price:,.2f}
24h Volume: {volume:,.0f}

Technical Indicators:
{json.dumps(indicators, indent=2)}

Recent News:
{chr(10).join(news) if news else "No recent news"}

Respond in JSON format with:
- sentiment: "bullish", "bearish", or "neutral"
- confidence: 0.0 to 1.0
- action: "BUY", "SELL", or "HOLD"
- reasoning: brief explanation

JSON:"""

        response = self.chat(
            prompt=prompt,
            seed=seed,
            system_message="You are an expert quantitative trader analyzing market data.",
        )
        
        # Parse JSON response
        return json.loads(response)
    
    @staticmethod
    def _hash_to_seed(text: str) -> int:
        """Convert text to deterministic seed."""
        return int(hashlib.sha256(text.encode()).hexdigest()[:16], 16) % (2**63)


class AIStrategy:
    """Base class for AI-powered trading strategies."""
    
    def __init__(self, api_key: str):
        self.ai = EigenAIClient(api_key=api_key)
        self.seed_mode = "deterministic"  # or "timestamp" for live trading
    
    def generate_signal(self, market_data) -> str:
        """Override this method with your AI logic."""
        raise NotImplementedError
    
    def get_seed(self, market_data) -> int:
        """Get seed based on mode."""
        if self.seed_mode == "deterministic":
            # Hash market data for reproducibility
            data_str = f"{market_data.symbol}_{market_data.timestamp}_{market_data.close}"
            return EigenAIClient._hash_to_seed(data_str)
        else:
            # Use timestamp for non-deterministic (live trading)
            return int(market_data.timestamp * 1e9) % (2**63)
```

### Option 3: Natural Language Strategy Builder

```python
class StrategyGenerator:
    """Generate trading strategies from natural language descriptions."""
    
    def __init__(self, ai_client: EigenAIClient):
        self.ai = ai_client
    
    def generate_from_description(self, description: str) -> str:
        """
        Convert natural language to executable Python strategy code.
        
        Example:
            description = '''
            Buy BTC when:
            - RSI < 30 (oversold)
            - Twitter sentiment turns positive
            - No major resistance levels within 5%
            
            Sell when:
            - 10% profit target hit
            - Sentiment turns negative
            - Stop loss at 3%
            '''
            
            code = generator.generate_from_description(description)
            # Returns valid Python Strategy class
        """
        prompt = f"""Generate a Python trading strategy class for the Neleus framework based on this description:

{description}

Requirements:
1. Inherit from neleus.Strategy
2. Implement on_data() method
3. Use self.buy(), self.sell() for orders
4. Include risk management (stop loss, take profit)
5. Add comments explaining logic

Return only the Python code, no markdown formatting.

Code:"""

        code = self.ai.chat(
            prompt=prompt,
            seed=self._hash_to_seed(description),
            system_message="You are an expert in trading algorithms and Python programming.",
        )
        
        return code
```

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Neleus Core                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Backtest    │  │   Live       │  │  Portfolio   │     │
│  │   Engine     │  │   Trading    │  │  Management  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│           │                │                  │             │
│           └────────────────┴──────────────────┘             │
│                            │                                │
│                    ┌───────▼────────┐                       │
│                    │  Strategy Bus  │                       │
│                    └───────┬────────┘                       │
│                            │                                │
└────────────────────────────┼────────────────────────────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
    ┌───────────▼───────────┐ ┌──────────▼──────────┐
    │  Traditional Strategy │ │   AI Strategy       │
    │  - RSI, MACD, etc     │ │   Adapter           │
    │  - Order book         │ │                     │
    │  - Volume profile     │ │   ┌──────────────┐  │
    └───────────────────────┘ │   │  EigenAI     │  │
                              │   │  Client      │  │
                              │   └──────┬───────┘  │
                              └──────────┼──────────┘
                                         │
                              ┌──────────▼──────────┐
                              │   EigenAI Service   │
                              │  - Deterministic    │
                              │  - Verifiable       │
                              │  - H100 GPUs        │
                              └─────────────────────┘
```

## User-Facing Features

### 1. CLI Commands

```bash
# Use AI for strategy generation
neleus ai generate --description "Mean reversion with sentiment filter"

# Backtest AI strategy (deterministic)
neleus backtest ai_strategy.py --ai-provider eigenai --seed 42

# Compare different AI prompts
neleus ai compare \
  --prompt-a "Conservative risk assessment" \
  --prompt-b "Aggressive growth mindset" \
  --asset BTC

# Verify past inference
neleus ai verify --trade-id 12345 --replay

# Generate strategy from description
neleus ai create --interactive
```

### 2. Configuration

**File**: `config.yaml`

```yaml
ai:
  provider: eigenai
  endpoint: https://api.eigenai.com/v1
  api_key: ${EIGENAI_API_KEY}
  model: meta-llama/Llama-3.3-70B-Instruct
  
  # Seed strategy for backtesting
  seed_strategy:
    mode: deterministic  # or timestamp, market_data
    fixed_seed: 42
  
  # Verification settings
  verification:
    enabled: true
    sample_rate: 0.1  # Verify 10% of inferences
    third_party: true

strategies:
  - name: ai_sentiment_momentum
    type: ai
    ai_config:
      prompt_template: |
        Analyze {symbol} market data:
        Price: ${price}
        RSI: {rsi}
        News: {news}
        
        Return: BUY, SELL, or HOLD with confidence level.
      
      system_message: "You are an expert trader specializing in momentum strategies."
      temperature: 0.0
```

### 3. Python Strategy Examples

**Example 1: Sentiment + Technical**

```python
from neleus import Strategy
from neleus.ai import EigenAIClient
import os

class SentimentMomentum(Strategy):
    def __init__(self):
        super().__init__()
        self.ai = EigenAIClient(api_key=os.getenv("EIGENAI_API_KEY"))
        self.position_size = 0.1
        
    def on_data(self, data):
        # Calculate technical indicators
        rsi = self.indicator('rsi', period=14)
        
        # Get AI sentiment analysis
        news = self.fetch_news(data.symbol)
        
        analysis = self.ai.analyze_market(
            symbol=data.symbol,
            price=data.close,
            volume=data.volume,
            indicators={"rsi": rsi},
            news=news,
            seed=self.get_deterministic_seed(data)
        )
        
        # Combine technical + AI sentiment
        if analysis['action'] == 'BUY' and analysis['confidence'] > 0.7 and rsi < 40:
            self.buy(size=self.position_size)
            self.set_stop_loss(percent=0.03)
            self.set_take_profit(percent=0.10)
            
        elif analysis['action'] == 'SELL' and analysis['confidence'] > 0.7 and rsi > 60:
            self.sell(size=self.position_size)
    
    def get_deterministic_seed(self, data):
        """Generate deterministic seed from market data."""
        import hashlib
        data_str = f"{data.symbol}_{data.timestamp}_{data.close}"
        return int(hashlib.sha256(data_str.encode()).hexdigest()[:16], 16)
```

**Example 2: Polymarket Event Betting**

```python
from neleus import Strategy
from neleus.ai import EigenAIClient

class PredictionMarketAgent(Strategy):
    def __init__(self):
        super().__init__()
        self.ai = EigenAIClient(api_key=os.getenv("EIGENAI_API_KEY"))
        self.venue = "polymarket"
        
    def monitor_events(self):
        """Monitor real-world events and place bets."""
        markets = self.get_active_markets(self.venue)
        
        for market in markets:
            # Fetch relevant news
            news = self.fetch_news_for_event(market.event_description)
            
            # AI analyzes event outcome probability
            prompt = f"""
            Event: {market.event_description}
            Current odds: YES={market.yes_price}, NO={market.no_price}
            
            Recent news:
            {chr(10).join(news)}
            
            Based on the news, estimate the probability this event occurs.
            Return JSON: {{"probability": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}}
            """
            
            analysis = self.ai.chat(
                prompt=prompt,
                seed=self._hash_news(news)  # Deterministic based on news
            )
            
            result = json.loads(analysis)
            
            # Place bet if AI is confident and odds are favorable
            if result['confidence'] > 0.8:
                expected_value = result['probability'] * 1.0 - (1 - result['probability']) * 1.0
                current_ev = market.yes_price * 1.0 - (1 - market.yes_price) * 1.0
                
                if expected_value > current_ev + 0.1:  # 10% edge
                    self.place_bet(
                        market=market.id,
                        outcome="YES",
                        size=self.kelly_criterion(result['probability'], market.yes_price)
                    )
                    
                    # Log for verification
                    self.log_inference(
                        market_id=market.id,
                        prompt=prompt,
                        response=analysis,
                        seed=self._hash_news(news)
                    )
```

**Example 3: Natural Language Strategy Generation**

```python
from neleus.ai import StrategyGenerator, EigenAIClient

# User describes strategy in plain English
description = """
Create a conservative mean reversion strategy for ETH:

Entry conditions:
- Price drops more than 2 standard deviations below 20-day moving average
- RSI < 25 (deeply oversold)
- Daily volume is above average
- No major negative news in last 24 hours

Exit conditions:
- Price returns to moving average
- 5% profit target reached
- Stop loss at 2% below entry

Position sizing:
- Risk 1% of portfolio per trade
- Maximum position: 20% of portfolio
"""

# Generate executable strategy code
generator = StrategyGenerator(EigenAIClient(api_key=api_key))
strategy_code = generator.generate_from_description(description)

# Code is deterministic - same description always generates same code
print(strategy_code)

# Save and use
with open("generated_strategy.py", "w") as f:
    f.write(strategy_code)

# Backtest generated strategy
# neleus backtest generated_strategy.py
```

### 4. Verification Feature

```python
class VerifiableStrategy(Strategy):
    def __init__(self):
        super().__init__()
        self.ai = EigenAIClient(api_key=os.getenv("EIGENAI_API_KEY"))
        self.inference_log = []
    
    def on_data(self, data):
        prompt = self.build_prompt(data)
        seed = self.compute_seed(data)
        
        # Make inference
        response = self.ai.chat(prompt=prompt, seed=seed)
        
        # Log for later verification
        self.inference_log.append({
            'timestamp': data.timestamp,
            'prompt': prompt,
            'seed': seed,
            'response': response,
            'model': self.ai.model,
        })
        
        # Execute trade
        signal = self.parse_signal(response)
        if signal == 'BUY':
            self.buy(size=0.1)
    
    def verify_past_trades(self):
        """Re-run all inferences and verify they produce same results."""
        for i, log in enumerate(self.inference_log):
            # Re-execute with same inputs
            new_response = self.ai.chat(
                prompt=log['prompt'],
                seed=log['seed']
            )
            
            # Verify bit-for-bit match
            if new_response == log['response']:
                print(f"✓ Trade {i} verified")
            else:
                print(f"✗ Trade {i} MISMATCH!")
                print(f"  Original: {log['response']}")
                print(f"  Replay:   {new_response}")
```

## Unique Value Propositions

### 1. Only Trading Framework with Deterministic AI Backtesting

**Problem**: Competitors using OpenAI/Anthropic APIs can't reproduce AI strategy results
- Backtest today: AI says "BUY" → +15% return
- Backtest tomorrow: AI says "HOLD" → +5% return
- **Same inputs, different outputs = unreliable backtesting**

**Solution**: Neleus + EigenAI
- Backtest is reproducible
- Share strategy with community
- Others can verify your results
- Regulatory compliance

### 2. Verifiable Algorithmic Trading

**Use Case**: Institutional Trading
- Compliance officer: "Prove this AI didn't hallucinate"
- With EigenAI: Re-run inference → Get identical result → Verified ✓
- Important for:
  - Regulatory audits
  - Client reporting
  - Risk management
  - Dispute resolution

### 3. Prediction Market Automation

**Natural Fit**:
- Neleus already supports Polymarket ✓
- EigenAI can analyze news deterministically ✓
- Build autonomous agents that:
  - Monitor news feeds
  - Classify event outcomes
  - Place bets when edge detected
  - **Prove reasoning if disputed**

**Example**: "Trump wins 2028 election" market
- Agent scrapes polls, news, social media
- EigenAI: "Based on data, 65% probability"
- Current market: YES @ 55%
- Edge detected → Place bet
- If dispute: Can prove AI used unbiased data & reasoning

### 4. Hybrid Strategies (Best of Both Worlds)

```
Fast Traditional Indicators (Rust):
  ✓ Sub-millisecond execution
  ✓ Order book analysis
  ✓ Technical indicators
  ✓ Always deterministic
  
+ Complex AI Reasoning (EigenAI):
  ✓ Natural language processing
  ✓ Multi-modal analysis
  ✓ Pattern recognition
  ✓ Now also deterministic!
  
= Optimal Strategy
```

**Example**: 
- Traditional: Detects breakout pattern (fast)
- AI: Confirms news sentiment supports move (slower but smarter)
- Combined: Higher conviction trades

### 5. Community Strategy Marketplace

With deterministic AI:
- Users can share AI strategies
- Others can verify performance claims
- "This strategy returned 50% in backtest"
- Anyone can replay and confirm
- Builds trust in AI trading systems

## Implementation Roadmap

### Phase 1: Minimal Viable Integration (Week 1)

**Goal**: Users can call EigenAI from Python strategies

**Tasks**:
1. Create `python/neleus/ai.py` with `EigenAIClient` class
2. Add example: `examples/ai_sentiment_strategy.py`
3. Documentation: `docs/AI_STRATEGIES.md`
4. Update `python/neleus/__init__.py` to export AI classes

**Deliverables**:
- Users can import: `from neleus.ai import EigenAIClient`
- Working example strategy
- 1 test example

**Effort**: ~8 hours

### Phase 2: Rust Adapter (Week 2)

**Goal**: Rust core can call EigenAI (for lower latency)

**Tasks**:
1. Create `crates/adapters-eigenai/`
2. Implement HTTP client with reqwest
3. Add seed management strategies
4. Async/await support
5. Python bindings via PyO3

**Deliverables**:
- Rust crate: `neleus-adapters-eigenai`
- Benchmarks: Compare Python vs Rust latency
- Integration tests

**Effort**: ~16 hours

### Phase 3: CLI Commands (Week 3)

**Goal**: User-friendly CLI for AI features

**Tasks**:
1. `neleus ai generate` - Generate strategy from description
2. `neleus ai backtest` - Backtest with AI
3. `neleus ai verify` - Verify past inferences
4. `neleus ai compare` - Compare different prompts

**Deliverables**:
- CLI commands working
- Documentation updated
- Tutorial video

**Effort**: ~12 hours

### Phase 4: Advanced Features (Week 4)

**Goal**: Production-ready features

**Tasks**:
1. Verification system (log all inferences)
2. Third-party verification API integration
3. Prometheus metrics for AI calls
4. Rate limiting & error handling
5. Cost tracking (token usage)

**Deliverables**:
- Production-grade reliability
- Monitoring & observability
- Cost optimization

**Effort**: ~20 hours

### Phase 5: Community & Documentation (Ongoing)

**Goal**: Adoption & ecosystem

**Tasks**:
1. Example strategies repository
2. Video tutorials
3. Blog posts / case studies
4. Integration guides
5. Community Discord channel for AI strategies

**Deliverables**:
- 10+ example strategies
- "AI Trading with Neleus" guide
- Community engagement

**Effort**: Ongoing

## Technical Considerations

### 1. Latency

**EigenAI Response Time**: ~1-3 seconds (typical for LLM inference)

**Implications**:
- ✓ Good for: Daily/hourly strategies, risk analysis, market regime classification
- ✗ Not for: High-frequency trading, sub-second signals

**Architecture**:
```python
# Async pattern for low latency
class HybridStrategy(Strategy):
    def on_data_fast(self, data):
        # Traditional indicators (microseconds)
        if self.detect_breakout():
            self.buy()  # Execute immediately
    
    async def on_data_slow(self, data):
        # AI analysis (seconds)
        sentiment = await self.ai.analyze_async(data)
        # Update global sentiment score
        self.sentiment_score = sentiment
```

### 2. Cost Management

**EigenAI Pricing**: 1M tokens free, then pay-per-token

**Optimization Strategies**:
1. **Cache Results**: Same prompt + seed = same output (no need to re-query)
2. **Rate Limiting**: Max N API calls per minute
3. **Batch Analysis**: Analyze multiple assets in one prompt
4. **Tiered Signals**: 
   - Quick signals: Traditional indicators (free)
   - Deep analysis: AI only when needed

```python
class CostOptimizedStrategy(Strategy):
    def __init__(self):
        self.ai_cache = {}  # Cache AI responses
        self.daily_api_limit = 100
        self.api_calls_today = 0
    
    def get_ai_signal(self, prompt, seed):
        cache_key = f"{hash(prompt)}_{seed}"
        
        # Check cache first
        if cache_key in self.ai_cache:
            return self.ai_cache[cache_key]
        
        # Check rate limit
        if self.api_calls_today >= self.daily_api_limit:
            return self.fallback_signal()
        
        # Make API call
        result = self.ai.chat(prompt, seed)
        self.ai_cache[cache_key] = result
        self.api_calls_today += 1
        
        return result
```

### 3. Error Handling

**EigenAI Failure Modes**:
- Network timeout
- Rate limiting
- API errors
- Malformed responses

**Resilience**:
```python
class ResilientAIStrategy(Strategy):
    def get_signal(self, data):
        try:
            # Try AI signal
            return self.ai.analyze(data, timeout=5.0)
        except TimeoutError:
            # Fallback to traditional
            return self.technical_signal(data)
        except Exception as e:
            self.log_error(e)
            return "HOLD"  # Conservative fallback
```

### 4. Seed Management

**Critical for Reproducibility**:

```python
# Bad: Non-deterministic (timestamp changes)
seed = int(time.time())

# Good: Deterministic (based on market data)
seed = hash(f"{symbol}_{timestamp}_{close_price}")

# Better: Explicit seed control
class Strategy:
    def __init__(self, seed_mode="deterministic"):
        self.seed_mode = seed_mode
        self.base_seed = 42
    
    def get_seed(self, data):
        if self.seed_mode == "deterministic":
            # Backtest: Same data = same seed
            return hash(f"{data.symbol}_{data.timestamp}")
        else:
            # Live: Use timestamp for variety
            return int(time.time_ns())
```

### 5. Testing Strategy

**Unit Tests**:
```python
def test_deterministic_signal():
    strategy = AIStrategy(seed_mode="deterministic")
    data = mock_market_data()
    
    signal1 = strategy.get_signal(data)
    signal2 = strategy.get_signal(data)
    
    assert signal1 == signal2  # Must be identical
```

**Integration Tests**:
- Test with real EigenAI API (use test account)
- Verify same prompt → same response
- Test error handling
- Test rate limiting

**Backtest Validation**:
- Run backtest twice → Results must match exactly
- Share backtest with teammate → They get same results
- Public reproducibility challenge

## Security & Privacy

### 1. API Key Management

```python
# Good: Environment variables
api_key = os.getenv("EIGENAI_API_KEY")

# Bad: Hardcoded
api_key = "sk-abc123..."  # Never do this!

# Best: Use secrets manager
from neleus.config import SecretManager
api_key = SecretManager.get("eigenai_api_key")
```

### 2. Prompt Data Sensitivity

**Concern**: Trading prompts contain alpha
- "I noticed BTC always drops after this pattern..."
- Sending to third-party API = leaking strategy

**Mitigation**:
1. **Data Minimization**: Only send necessary data
2. **Prompt Templates**: Use generic templates, fill in values
3. **Self-Hosting**: EigenAI open-sources their stack (future)
4. **Encryption**: HTTPS + TLS 1.3

```python
# Avoid: Specific strategy details in prompt
prompt = "I discovered BTC drops 5% every time RSI>70 and Elon tweets..."

# Better: Generic analysis request
prompt = "Analyze market sentiment given RSI={rsi} and news={news}"
```

### 3. Verification Trust

**Question**: How do we trust EigenAI's verification?

**Answer**: Open-source inference stack
- Anyone with H100 GPU can re-run
- Verification operators can attest
- Eventually: Cryptographic proofs (ZK?)

## Competitive Advantages

| Feature | Neleus + EigenAI | Competitors |
|---------|------------------|-------------|
| AI Strategy Backtesting | ✓ Deterministic | ✗ Non-reproducible |
| Verifiable Trading Decisions | ✓ Cryptographic proof | ✗ Trust required |
| Rust Performance | ✓ Sub-ms indicators | ~ Varies |
| Multi-Venue Support | ✓ CEX + DEX | ~ Limited |
| Polymarket Integration | ✓ Native | ✗ Not available |
| Open Source | ✓ MIT License | ~ Mixed |
| Python + Rust Hybrid | ✓ Best of both | ✗ Usually one or other |
| Prediction Markets AI | ✓ Unique positioning | ✗ No one else |

## Conclusion

Integrating EigenAI into Neleus creates a **unique value proposition** in the trading framework space:

1. **Only framework with deterministic AI backtesting**
2. **Only framework with verifiable trading decisions**
3. **Only framework combining Rust performance + AI intelligence + DeFi/Polymarket support**

**Near-term wins**:
- Easy integration (OpenAI-compatible API)
- Immediate value for users (AI-powered strategies)
- Differentiation from competitors

**Long-term vision**:
- Verifiable AI trading becomes industry standard
- Neleus becomes the go-to platform for AI + DeFi trading
- Community of AI strategy developers

**Next steps**:
1. Implement Phase 1 (Python client + examples)
2. Create demo video showing deterministic backtest
3. Reach out to EigenAI for partnership
4. Launch with blog post: "Reproducible AI Trading with Neleus + EigenAI"

---

## Resources

- **EigenAI Docs**: https://docs.eigen.ai
- **EigenAI API**: https://api.eigenai.com/v1
- **deTERMinal**: https://determinal.ai (1M free tokens)
- **OpenAI SDK**: Compatible out-of-the-box
- **Example Code**: See `examples/ai_*.py`

## Appendix: Example Prompts

### Trading Signal Generation
```
Analyze the following cryptocurrency market data:

Symbol: {symbol}
Current Price: ${price}
24h Change: {change_percent}%
Volume: {volume}
RSI(14): {rsi}
MACD: {macd}
Bollinger Bands: {bb_upper}, {bb_lower}

Recent News Headlines:
{news}

Based on technical indicators and news sentiment:
1. Provide a trading signal: BUY, SELL, or HOLD
2. Confidence level: 0-100%
3. Brief reasoning (2-3 sentences)

Respond in JSON format:
{
  "signal": "BUY|SELL|HOLD",
  "confidence": 0-100,
  "reasoning": "..."
}
```

### Risk Assessment
```
Evaluate the risk level of the following portfolio:

Positions:
{position_list}

Portfolio Metrics:
- Total Value: ${total_value}
- Unrealized P&L: ${unrealized_pnl}
- Daily P&L: ${daily_pnl}
- Max Drawdown: {max_drawdown}%
- Sharpe Ratio: {sharpe}

Market Conditions:
- VIX: {vix}
- BTC 30-day volatility: {btc_vol}%
- Market correlation: {correlation}

Risk Limits:
- Max daily loss: ${max_daily_loss}
- Max position size: ${max_position}

Assess:
1. Overall risk level: LOW, MEDIUM, HIGH, CRITICAL
2. Specific risks identified
3. Recommended actions

JSON:
{
  "risk_level": "LOW|MEDIUM|HIGH|CRITICAL",
  "risks": ["risk1", "risk2"],
  "recommendations": ["action1", "action2"]
}
```

### Market Regime Classification
```
Classify the current market regime based on:

Price Action (30 days):
- Returns: {returns}
- Volatility: {volatility}
- Trend strength: {trend}

Market Microstructure:
- Bid-ask spread: {spread}
- Order book depth: {depth}
- Trade frequency: {frequency}

Macro Context:
- Interest rates: {rates}
- Economic data: {econ_data}

Classify as one of:
1. TRENDING_UP
2. TRENDING_DOWN
3. MEAN_REVERTING
4. HIGH_VOLATILITY
5. LOW_VOLATILITY
6. RANGING

JSON:
{
  "regime": "...",
  "confidence": 0-100,
  "characteristics": ["..."]
}
```
