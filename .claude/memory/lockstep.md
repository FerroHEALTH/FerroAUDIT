---
name: lockstep
description: "Every product pins ihe-atna exactly from crates.io; a release that changes an API rolls out as one pull request per consuming product, each keeping every test"
metadata:
  type: project
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Every FerroHEALTH product that audits (FerroFED, FerroEHR, FerroPIX)
depends on `ihe-atna` pinned exactly (`=X.Y.Z`)
from crates.io, never a submodule or a path dependency, the way the products
depend on `ferrohealth-smart` (FerroAUTH) and FerroUI.

- **One pull request per product.** A release a product must take is rolled
  out as a pin-bump or migration pull request in each consuming repository,
  coordinated from the orchestrating session, each green on its own gates
  and keeping every existing test.
- **Shared, never single use.** A behaviour a product needs and FerroAUDIT
  lacks is filed on FerroAUDIT and built there, never as a local copy.

**How to apply:** a product's migration changes only the pin, the seams it
plugs in and the renames the changelog names; anything else found en route
is filed on the product's tracker.
