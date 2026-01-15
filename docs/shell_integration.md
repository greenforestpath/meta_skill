# Shell Integration (ms suggest hooks)

This project provides a lightweight shell hook that calls `ms suggest` and relies on cooldowns + context fingerprints to avoid spam.

## Quick Start

Print the hook for your shell:

```bash
ms shell --shell bash
ms shell --shell zsh
ms shell --shell fish
```

Then paste the snippet into your shell rc file:

- bash: `~/.bashrc`
- zsh: `~/.zshrc`
- fish: `~/.config/fish/config.fish`

Restart your shell after adding the snippet.

## Rate Limiting

The hook uses a time gate to reduce overhead. You can override the interval:

```bash
# seconds between suggestions
export MS_SUGGEST_INTERVAL=30
```

## Removal

Delete the snippet from your shell rc file and restart the shell.

## Notes

- `ms suggest` respects cooldowns automatically.
- You can temporarily disable cooldowns with `ms suggest --ignore-cooldowns`.
- For better suggestions, set `MS_OPEN_FILES` to a comma-separated list of file paths.
