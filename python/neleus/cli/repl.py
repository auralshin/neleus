import shlex
import typer
import click
from prompt_toolkit import PromptSession
from prompt_toolkit.history import InMemoryHistory
from rich.console import Console

console = Console()

def start_repl(app: typer.Typer) -> None:
    """Start an interactive chat-like REPL session for the CLI."""
    session = PromptSession(history=InMemoryHistory())

    console.print("[bold cyan]Welcome to Neleus CLI.[/bold cyan]")
    console.print("Type [bold yellow]--help[/bold yellow] to see available commands, or [bold yellow]exit[/bold yellow] to quit.\n")

    click_command = typer.main.get_command(app)

    while True:
        try:
            text = session.prompt("neleus > ")
            if not text.strip():
                continue

            # Remove leading slash if the user treats it like an AI CLI slash-command
            if text.startswith("/"):
                text = text[1:]

            stripped = text.strip().lower()
            if stripped in ("exit", "quit"):
                console.print("[green]Goodbye![/green]")
                break

            # Special-case "help" → show top-level help
            if stripped == "help":
                ctx = click.Context(click_command, info_name="neleus")
                console.print(click_command.get_help(ctx))
                continue

            try:
                args = shlex.split(text)
            except ValueError as e:
                console.print(f"[red]Parse error:[/red] {e}")
                continue

            try:
                click_command.main(args=args, prog_name="neleus", standalone_mode=False)
            except click.exceptions.Exit:
                pass
            except click.exceptions.Abort:
                console.print("[yellow]Aborted.[/yellow]")
            except click.exceptions.UsageError as e:
                e.show()
            except SystemExit:
                pass
            except Exception as e:
                console.print(f"[red]Error executing command:[/red] {e}")

        except KeyboardInterrupt:
            # Ctrl+C clears the prompt
            continue
        except EOFError:
            # Ctrl+D exits
            console.print("\n[green]Goodbye![/green]")
            break
