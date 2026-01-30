import asyncio
import json
import os
from pathlib import Path
from typing import Optional
import http.server
import socketserver
import webbrowser
import threading


__all__ = [
    "start_dashboard",
    "DashboardHandler",
]

DASHBOARD_HTML = Path(__file__).parent / "dashboard.html"


class DashboardHandler(http.server.SimpleHTTPRequestHandler):
                                         
    
    def __init__(self, *args, project_path: Optional[Path] = None, **kwargs):
        self.project_path = project_path or Path.cwd()
        super().__init__(*args, **kwargs)
    
    def do_GET(self):
                                  
        if self.path == "/" or self.path == "/index.html":
            self.send_response(200)
            self.send_header("Content-type", "text/html")
            self.end_headers()
            with open(DASHBOARD_HTML, "rb") as f:
                self.wfile.write(f.read())
        
        elif self.path == "/api/config":
            self.send_json(self._get_config())
        
        elif self.path == "/api/strategies":
            self.send_json(self._get_strategies())
        
        elif self.path == "/api/status":
            self.send_json(self._get_status())
        
        else:
            super().do_GET()
    
    def do_POST(self):
                                   
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)
        
        if self.path == "/api/config":
            data = json.loads(body)
            self._save_config(data)
            self.send_json({"status": "ok"})
        
        elif self.path == "/api/backtest":
            data = json.loads(body)
            result = self._run_backtest(data)
            self.send_json(result)
        
        else:
            self.send_error(404)
    
    def send_json(self, data):
                                 
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())
    
    def _get_config(self):
                                        
        config_path = self.project_path / "config.yaml"
        if config_path.exists():
            import yaml
            with open(config_path) as f:
                return yaml.safe_load(f)
        return {}
    
    def _get_strategies(self):
                                     
        strategies_dir = self.project_path / "strategies"
        if not strategies_dir.exists():
            return []
        
        strategies = []
        for f in strategies_dir.glob("*.py"):
            if not f.name.startswith("_"):
                strategies.append({
                    "name": f.stem,
                    "file": str(f),
                    "status": "stopped",
                })
        return strategies
    
    def _get_status(self):
                                 
        return {
            "connected": True,
            "mode": "paper",
            "uptime": 0,
        }
    
    def _save_config(self, data):
                                 
                                       
        pass
    
    def _run_backtest(self, data):
                           
                                         
        return {
            "total_return": 0.245,
            "sharpe_ratio": 2.14,
            "max_drawdown": -0.083,
            "win_rate": 0.62,
            "total_trades": 147,
            "profit_factor": 1.85,
        }
    
    def log_message(self, format, *args):
                                       
        pass


def create_handler(project_path: Optional[Path] = None):
                                                 
    class Handler(DashboardHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, project_path=project_path, **kwargs)
    return Handler


def start_dashboard(
    host: str = "127.0.0.1",
    port: int = 8080,
    project_path: Optional[Path] = None,
    open_browser: bool = True,
):
\
\
\
\
\
\
\
\
       
    handler = create_handler(project_path)
    
    with socketserver.TCPServer((host, port), handler) as httpd:
        url = f"http://{host}:{port}"
        print(f"Dashboard running at {url}")
        print("Press Ctrl+C to stop")
        
        if open_browser:
            webbrowser.open(url)
        
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nDashboard stopped")


if __name__ == "__main__":
    start_dashboard()
