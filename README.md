# Trails Save Editor

**English** · [Russian](README_RU.md)

Save editors for The Legend of Heroes series (PC, Steam):

| Program | Games |
|---|---|
| **SE-Calvard** (`Calvard-SaveEditor.exe`) | Trails through Daybreak, Trails through Daybreak II, Trails beyond the Horizon |
| **SE-Crossbell** (`Crossbell-SaveEditor.exe`) | Trails from Zero, Trails to Azure |

The interface is in English.

## Features

- **General** — mira, sepith, difficulty, play time; for Calvard also Total SP, the Arkride Solutions rank and the L.G.C. gauges.
- **Party** — who is in battle, in reserve and who can be added: drag the cards or click two of them to swap.
- **Items** — the amount of any item, with the game's icons and categories; search by name.
- Search across all tabs (Ctrl+F), light and dark theme.
- The save checksum is recalculated automatically.
- Before writing, the slot is backed up to the `Backups` folder next to the program.

## How to use

1. Download the archive from [Releases](../../releases) and unpack it anywhere.
   `data.dat` must stay next to the exe: it holds the item icons, portraits and game tables — without it the editor can't show items or characters.
2. Run the editor, choose the game and a save slot.
3. Make your changes and click «Save to this slot» or «Save to new slot».

Close the game or return to the title screen before writing. Keep your own copies of your saves.

## Building from source

You need Windows, [Rust](https://rustup.rs) and Visual Studio Build Tools (the C++ compiler and the Windows SDK — for the exe icons).

```
cargo build --release
```

`Calvard-SaveEditor.exe` and `Crossbell-SaveEditor.exe` end up in `target\release`.
The repository contains no game data: put `data.dat` from the release archive next to the exe.

## Rights

The source code is published for reference only; no license to use or modify it is granted — all rights reserved by the author.

The button icons (`src/icons/*.svg`, except `calvard_save.svg` and `crossbell_save.svg`) are from
[Lucide](https://lucide.dev) and are licensed under the ISC License — see `src/icons/LICENSE`.

The Legend of Heroes, Trails through Daybreak, Trails beyond the Horizon, Trails from Zero and Trails to Azure
are trademarks of Nihon Falcom Corporation. This project is not affiliated with or endorsed by Nihon Falcom.
The item icons, portraits and game texts in `data.dat` belong to Nihon Falcom.
