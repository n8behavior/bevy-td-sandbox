# Editor Compatibility

The game ships with an in-game **tower editor** — a visual UI for assembling towers without writing Lua. Editor and modding system speak the same language: a tower designed in the editor saves as a Lua recipe in `assets/towers/`, and a hand-written recipe loads into the editor for visual editing.

## Round-trip: editor ↔ Lua

When the editor saves, it writes a canonical `Tower { ... }` recipe:

- Identity fields ordered: `name`, `cost`, `color`, optional fields after.
- Deliverer blocks first, then passives.
- Property fields inside each block in a stable order.
- One block per visible group, conventional spacing.

Re-saving an unchanged tower produces a byte-identical file. Hand-written recipes don't have to match the editor's formatting — load is forgiving — but matching it keeps diffs clean across editor saves.

## What round-trips fully

A flat `Tower { ... }` literal:

```lua
return Tower {
  name = "...", cost = N, color = "...",
  DelivererBlock { ... },
  Passive(),
}
```

Open it in the editor, change any value, save — same shape back. Block order and property order may be canonicalized, but no information is lost.

## What round-trips partially

Recipes that return a *list* of towers:

```lua
return {
  Tower { name = "A", ... },
  Tower { name = "B", ... },
}
```

The editor opens each tower individually. Saving rewrites the whole file in canonical form, preserving the other towers but losing intermediate comments or formatting.

## What doesn't round-trip

Recipes that use functions, loops, `table.unpack`, or other Lua features ([Advanced Patterns](advanced.md)). The editor *loads* them — it runs the recipe and reads the resulting Towers — but it can't save them back as functions. Saving overwrites the file with flattened literals; helpers are gone.

This is deliberate. The editor is a flat-tower tool; expressive Lua is for hand-editing.

## Editability indicator

The editor shows the file's editability status in the corner of the edit panel:

- **Green: Round-trip safe.** Edits save back losslessly.
- **Yellow: Multi-tower file.** Edits save back; file is rewritten in canonical form.
- **Red: Read-only.** Recipe used computed values, helpers, or loops. You can browse and even tune parameters in preview mode, but you can't save changes back. The "Save" button becomes "Save as new file" — it writes the flattened result to a new `.lua` file, leaving the original alone.

## Writing for both surfaces

To author a tower that's both editor-tweakable and clean source:

1. **One file, one tower.** Don't return a list.
2. **No helpers, no loops, no computed identity.** Write blocks out longhand.
3. **Comments survive but migrate** to the end of the file on save. Don't rely on placement.

If you don't care about editor round-tripping — you're writing a pack as a developer — use whatever Lua features make the file pleasant to maintain.
