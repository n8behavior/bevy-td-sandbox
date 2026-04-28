# Editor Compatibility

The game ships with an in-game **tower editor** — a visual UI for assembling towers without writing Lua. The editor and the modding system speak the same language: a tower designed in the editor saves as a Lua recipe in `assets/towers/`, and a hand-written recipe loads into the editor for visual editing.

This page explains where that round-trip works perfectly, where it doesn't, and how to write recipes that stay editor-friendly.

## Round-trip: editor ↔ Lua

When you save a tower from the editor, it writes a flat `Tower { ... }` recipe with:

- Identity fields ordered: `name`, `cost`, `color`, optional fields after.
- Atoms grouped by palette (Triggers first, then Acquirers, Deliverers, Payloads, Modifiers, then structural).
- One atom per line, conventional spacing.

The editor's saved recipes are reproducible: re-saving an unchanged tower produces a byte-identical file. Hand-written recipes don't have to follow the editor's formatting — load is forgiving — but if you want diffs to stay clean across editor saves, matching the editor's style is easiest.

## What round-trips fully

Any recipe that's a flat `Tower { ... }` literal:

```lua
return Tower {
    name = "...", cost = N, color = "...",
    Atom1(...),
    Atom2(...),
    -- ...
}
```

Open it in the editor, change any value, save — you get back the same shape. Atom order may be canonicalized (the editor groups by palette), but no information is lost.

## What round-trips partially

Recipes that return a *list* of towers from one file:

```lua
return {
    Tower { name = "A", ... },
    Tower { name = "B", ... },
}
```

The editor opens each tower individually. Saving one tower from the file rewrites the *whole file* — preserving the other towers in the list, but in canonicalized form. If the file had comments, formatting, or computed identity fields between the towers, they're lost.

## What doesn't round-trip

Recipes that use functions, loops, `table.unpack`, or other Lua-the-language features ([Advanced Patterns](advanced.md)):

```lua
local function make_sniper(tier, cost, ...) ... end

return {
    make_sniper("I",   100, ...),
    make_sniper("II",  200, ...),
}
```

The editor can *load* these — it runs the recipe and reads the resulting Tower values. But it can't *save* them back as functions. If you save from the editor, the file is overwritten with two flat `Tower { ... }` literals, and the `make_sniper` helper is gone.

This is a deliberate tradeoff. The editor is a flat-tower tool; expressive Lua is for hand-editing. If a recipe matters to you as source code (one helper feeding many towers), open it in your text editor, not the in-game editor.

## How to know which mode you're in

When the editor loads a recipe, it shows the file's editability status in the corner of the edit panel:

- **Green: Round-trip safe.** Edits will save back without losing structure.
- **Yellow: Multi-tower file.** Edits save back; the file is rewritten in canonical form.
- **Red: Read-only.** The recipe used computed values, helpers, or loops. You can browse atoms, see the runtime behavior, even tune parameters in a "preview" mode — but you can't save changes back.

In red mode, the editor's "Save" button becomes "Save as new file" — it'll write the flattened result to a new `.lua` file, leaving the original alone.

## Writing for both surfaces

If you're authoring a tower you want to be both *editor-tweakable by players* and *clean source code for yourself*, the rules are simple:

1. **One file, one tower.** Keep families across multiple files instead of returning a list.
2. **No helpers, no loops, no computed identity.** Write the atoms out longhand.
3. **Comments survive in the editor**, but they migrate to the end of the file when saved. Don't depend on a comment being on a specific line.

If you don't care about editor round-tripping — you're writing a recipe pack as a developer, not as a player — ignore these rules and use whatever Lua features make the file pleasant to maintain. The recipe loader doesn't care.
