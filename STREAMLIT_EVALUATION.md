# Streamlit Frontend Evaluation for Neleus

## Executive Summary

**Recommendation: YES - Switch to Streamlit for MVP/Demo phase**

Switching to Streamlit would:
-  Reduce frontend code by ~80% (3300+ lines → ~500 lines)
-  Make financial charts and risk visualization much simpler
-  Speed up iteration for demo/MVP significantly
-  Enable better data visualization out of the box
- ⚠️ Trade some customization flexibility for speed

---

## Current State Analysis

### Existing FastAPI + HTML/JS Dashboard

| Metric | Value |
|--------|-------|
| Total Lines | 3,369 lines |
| HTML/JS Code | ~3,000 lines (embedded) |
| API Endpoints | 29 functions |
| Dashboards | 2 (Trading + Managed Service) |
| Dependencies | FastAPI, uvicorn, vanilla JS, Chart.js |

**Complexity:**
- Custom HTML/CSS/JS for both dashboards
- Manual state management and API calls
- Chart.js for visualizations
- WebSocket for real-time updates

---

## Streamlit Comparison

### What Streamlit Provides

| Feature | Current (FastAPI+HTML) | Streamlit |
|---------|----------------------|-----------|
| **Code Volume** | 3,300+ lines | ~500 lines |
| **Charts** | Manual Chart.js | Built-in Plotly/Altair |
| **Tables** | HTML tables | `st.dataframe()` |
| **Forms** | Manual HTML forms | `st.form()` |
| **Layouts** | CSS Grid/Flex | `st.columns()`, `st.tabs()` |
| **State Management** | Manual JS | Automatic |
| **Real-time Updates** | WebSocket | `st.experimental_rerun()` |
| **Deployment** | Separate server | Single command |

### Example: Agent Table

**Current (HTML/JS):** ~150 lines
```javascript
function renderAgents() {
    const tbody = document.getElementById('agents-tbody');
    const rows = agents.map(agent => {
        const pnlClass = agent.pnl.total >= 0 ? 'positive' : 'negative';
        return `
            <tr>
                <td><div class="agent-name">${agent.name}</div></td>
                <td><span class="state-badge ${agent.state}">● ${agent.state}</span></td>
                // ... many more lines
            </tr>
        `;
    }).join('');
    tbody.innerHTML = rows;
}
```

**Streamlit:** ~10 lines
```python
import streamlit as st
import pandas as pd

agents_df = pd.DataFrame(agents)
st.dataframe(
    agents_df,
    column_config={
        "pnl": st.column_config.NumberColumn("P&L", format="$%.2f"),
        "state": st.column_config.Column("State", help="Agent status")
    }
)
```

---

## Impact Assessment

###  Positive Impacts

1. **Development Speed** 🚀
   - 5-10x faster to build/modify dashboards
   - No HTML/CSS/JS knowledge needed
   - Focus on Python only

2. **Financial Charts** 📊
   - Plotly charts with built-in finance templates
   - Candlestick charts: `plotly.graph_objects.Candlestick`
   - Area charts for P&L curves
   - Built-in interactivity (zoom, pan, hover)

3. **Risk Metrics** ⚠️
   - Easy metric cards: `st.metric("P&L", "$1,250", delta="+15%")`
   - Progress bars for risk limits
   - Color-coded alerts
   - Table styling with conditions

4. **Iteration Speed** ⚡
   - Change code → auto-refresh
   - No build step
   - Hot reload during development

5. **Demo Quality** 🎬
   - Professional-looking out of the box
   - Better data viz than custom HTML
   - Interactive by default

### ⚠️ Trade-offs

1. **Customization**
   - Less control over exact CSS/layout
   - Streamlit's opinionated design
   - Can't embed complex JS widgets easily

2. **Performance**
   - Streamlit reruns entire script on interaction
   - Not ideal for high-frequency updates (but fine for demo)
   - WebSocket is more efficient for real-time

3. **Architecture**
   - Streamlit apps are separate from FastAPI
   - Need to keep API endpoints separate
   - Can run side-by-side or replace entirely

---

## Implementation Difficulty

### Effort Estimate

| Task | Current Lines | Streamlit Lines | Difficulty | Time |
|------|--------------|----------------|------------|------|
| **Managed Service Dashboard** | ~1,000 | ~200 | Easy | 2-3 hours |
| **Trading Dashboard** | ~2,000 | ~300 | Easy | 3-4 hours |
| **API Integration** | Keep as-is | Keep as-is | None | 0 hours |
| **Deployment Setup** | Existing | New | Easy | 1 hour |
| **Total** | 3,000+ | ~500 | **Easy** | **6-8 hours** |

### Migration Path

#### Option 1: Full Replacement (Recommended for MVP)
```
Old: FastAPI server with embedded HTML
New: Streamlit app + FastAPI API (separate)

Benefits:
- Clean separation of concerns
- Easier to maintain
- Better for demo/MVP

Structure:
python/neleus/ui/
├── streamlit_app.py          # Main Streamlit app
├── pages/
│   ├── 1_📊_Trading.py        # Trading dashboard
│   ├── 2_🤖_Agents.py         # Agent management
│   └── 3_📡_Signals.py        # Signal feed
└── api_client.py              # Wrapper for API calls
```

#### Option 2: Hybrid Approach
Keep FastAPI for API endpoints, add Streamlit for visualization:
```
Run both:
- FastAPI on :8765 (API only)
- Streamlit on :8501 (Dashboard)
```

---

## Code Examples

### 1. Managed Service Overview

**Streamlit Version:**
```python
import streamlit as st
import requests

st.set_page_config(page_title="Neleus - Managed Service", layout="wide")

# Header
st.title("🌊 Neleus Managed Service")

# Fetch data
overview = requests.get("http://localhost:8765/api/overview").json()

# Metrics row
col1, col2, col3, col4 = st.columns(4)
with col1:
    st.metric(
        "Total P&L (Today)",
        f"${overview['total_pnl']['today']:.2f}",
        delta=f"+{12.5}%"
    )
with col2:
    st.metric(
        "Active Agents",
        f"{overview['agents']['running']} / {overview['agents']['total']}"
    )
with col3:
    st.metric(
        "Signals (24h)",
        overview['signals']['received_24h']
    )
with col4:
    st.metric(
        "Avg Latency",
        f"{overview['system']['latency_avg_ms']}ms"
    )

# Agents table
st.subheader("🤖 Deployed Agents")
agents = requests.get("http://localhost:8765/api/agents").json()
df = pd.DataFrame(agents)

# Style based on state
def color_state(val):
    if val == 'running':
        return 'background-color: rgba(16, 185, 129, 0.15)'
    elif val == 'paused':
        return 'background-color: rgba(245, 158, 11, 0.15)'
    return ''

styled_df = df.style.applymap(color_state, subset=['state'])
st.dataframe(styled_df, width='stretch')

# Agent controls
selected_agent = st.selectbox("Select Agent", df['id'])
col1, col2, col3 = st.columns(3)
with col1:
    if st.button("▶️ Start"):
        requests.post(f"http://localhost:8765/api/agents/{selected_agent}/start")
        st.success("Agent started")
with col2:
    if st.button("⏸️ Pause"):
        requests.post(f"http://localhost:8765/api/agents/{selected_agent}/pause")
        st.warning("Agent paused")
with col3:
    if st.button("⏹️ Stop"):
        requests.post(f"http://localhost:8765/api/agents/{selected_agent}/stop")
        st.error("Agent stopped")
```

**Result:** 50 lines vs 1000+ lines, same functionality

---

### 2. P&L Chart

**Streamlit Version:**
```python
import plotly.graph_objects as go

# Fetch P&L history
metrics = requests.get(f"http://localhost:8765/api/agents/{agent_id}/metrics").json()
history = metrics['history']

# Create line chart
fig = go.Figure()
fig.add_trace(go.Scatter(
    x=[h['time'] for h in history],
    y=[h['pnl'] for h in history],
    mode='lines+markers',
    name='P&L',
    line=dict(color='#10b981', width=2),
    fill='tozeroy',
    fillcolor='rgba(16, 185, 129, 0.1)'
))

fig.update_layout(
    title="P&L History",
    xaxis_title="Time",
    yaxis_title="P&L ($)",
    hovermode='x unified',
    template='plotly_dark'
)

st.plotly_chart(fig, width='stretch')
```

**Result:** Clean, interactive chart in 20 lines

---

### 3. Risk Metrics Dashboard

**Streamlit Version:**
```python
st.subheader("⚠️ Risk Metrics")

risk = metrics['risk']

col1, col2 = st.columns(2)

with col1:
    # Drawdown gauge
    st.metric("Max Drawdown", f"{risk['max_drawdown']:.1%}")
    st.progress(risk['max_drawdown'])
    
    # Sharpe ratio
    st.metric("Sharpe Ratio", f"{risk['sharpe']:.2f}")
    
with col2:
    # VaR
    st.metric("VaR (95%)", f"${risk['var_95']:,.2f}")
    
    # Sortino
    st.metric("Sortino Ratio", f"{risk['sortino']:.2f}")

# Risk alerts
if risk['max_drawdown'] > 0.10:
    st.error("⚠️ Drawdown exceeds 10% threshold")
elif risk['max_drawdown'] > 0.05:
    st.warning("⚠️ Drawdown approaching limit")
else:
    st.success(" Risk within acceptable limits")
```

---

## For Demo/MVP: Streamlit is Perfect

### Why Streamlit Wins for Demo

1. **Speed** - Build dashboard in hours not days
2. **Polish** - Looks professional out of the box
3. **Charts** - Financial visualizations are Streamlit's strength
4. **Simplicity** - One Python file per page
5. **Iteration** - Change code, instantly see results

### Recommended Architecture for Demo

```
┌─────────────────────────────────────────────────────────────┐
│                     Neleus Demo Stack                        │
│                                                              │
│  ┌────────────────────┐         ┌──────────────────────┐   │
│  │  Streamlit UI      │────────▶│  FastAPI Backend     │   │
│  │  Port: 8501        │  HTTP   │  Port: 8765          │   │
│  │                    │         │  (API endpoints)     │   │
│  │  - Agent Dashboard │         │  /api/agents         │   │
│  │  - Metrics Charts  │         │  /api/signals        │   │
│  │  - Signal Feed     │         │  /api/metrics        │   │
│  └────────────────────┘         └──────────────────────┘   │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Python Trading Core                       │ │
│  │  (agents, signals, backtest, etc.)                    │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Launch Commands

```bash
# Terminal 1: API Server
neleus ui --api-only

# Terminal 2: Streamlit Dashboard
streamlit run python/neleus/ui/streamlit_app.py

# Or combined (future):
neleus ui --streamlit
```

---

## Migration Plan

### Phase 1: Create Streamlit Version (2-3 hours)
1. Create `streamlit_app.py` with managed service dashboard
2. Implement agent management page
3. Add P&L and metrics visualization
4. Keep existing API endpoints as-is

### Phase 2: Polish for Demo (1-2 hours)
1. Add proper styling and colors
2. Implement signal feed page
3. Add real-time updates
4. Test all interactions

### Phase 3: Demo Ready (1 hour)
1. Update product demo guide
2. Add Streamlit to requirements
3. Update CLI to launch Streamlit

**Total: 4-6 hours** to have production-quality Streamlit dashboard

---

## Dependencies

### Add to requirements.txt
```txt
streamlit>=1.30.0
plotly>=5.18.0
pandas>=2.0.0
requests>=2.31.0
```

### Install
```bash
pip install streamlit plotly
```

---

## Recommendation

### For Demo/MVP Stage: **USE STREAMLIT**

**Reasons:**
1. ⏱️ **Time to Demo**: 6-8 hours vs 20+ hours for custom HTML
2. 📊 **Chart Quality**: Better financial charts out of the box
3. 🔧 **Maintainability**: Much easier to update/modify
4. 🎨 **Polish**: Professional look without CSS expertise
5. 🚀 **Iteration**: Instant feedback loop

### After Demo (Production):
You can always:
- Keep Streamlit if it works well
- Migrate to custom React/Vue if needed
- Use Streamlit for internal tools, custom UI for customers

---

## Next Steps

If you decide to proceed with Streamlit:

1. I can create the full Streamlit dashboard (4-6 hours of work)
2. Keep all existing API endpoints
3. Update product demo guide for Streamlit
4. Test and polish for recording

**Decision needed:**
-  Yes, build Streamlit version for demo
- ❌ No, keep current FastAPI+HTML dashboard
- 🤔 Build both, compare side-by-side

Let me know which path you prefer!
