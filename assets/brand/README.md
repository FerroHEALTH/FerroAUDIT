<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# FerroAUDIT brand

FerroAUDIT is a shared library of the FerroHEALTH family, not a product. The
family's brand rule gives the shared libraries (FerroAUTH, FerroAUDIT,
FerroUI) no hue and no stroke of their own and shows them in the parent's
steel. The lockups here follow that rule: the family pulse drawn in one
steel stroke, and the wordmark set as FerroHEALTH's is, "Ferro" in iron and
"AUDIT" in steel.

The source is FerroHEALTH's `assets/brand/ferrohealth-lockup-auto.svg` and
the palette in its `assets/brand/tokens.css`: the pulse path, the type and
the layout are taken from there, with the four product colours replaced by
steel. Everything here is under the Business Source License 1.1 with the
rest of the repository.

| Token | Hex | Use |
|---|---|---|
| iron | `#0F172A` | "Ferro" on a light ground |
| mist | `#F1F5F9` | "Ferro" on a dark ground |
| steel | `#3F6070` | the pulse and "AUDIT" on a light ground |
| steel-light | `#8FB6C4` | the pulse and "AUDIT" on a dark ground |

## Files

| File | What it is |
|---|---|
| `ferroaudit-lockup-auto.svg` | the lockup for the README and any page that serves one file to both themes, switched on `prefers-color-scheme` |
| `ferroaudit-lockup-light.svg` | the lockup for a light ground |
| `ferroaudit-lockup-dark.svg` | the lockup for a dark ground |
