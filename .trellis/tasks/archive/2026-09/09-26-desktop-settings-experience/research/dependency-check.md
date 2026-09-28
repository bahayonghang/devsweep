# Approved Dependency Check

Checked on 2026-09-26 before installation. The user approved these two direct production dependencies and sequential implementation.

- npm registry reports @base-ui/react 1.8.0, MIT license, React and react-dom peers ^17 || ^18 || ^19. The date-fns and @date-fns/tz peers are optional; Settings does not need date components. Install only the scoped Select/Combobox package usage and normal required transitives. Registry unpacked size is 9,628,362 bytes; this is not a browser bundle-size measurement.
- Cargo.lock already contains windows 0.61.3 (and an older transitive 0.56.0). Prefer the already locked compatible version for the new Windows-only direct DirectWrite dependency after confirming its MSRV/features. Do not add another font library.
- Source commands: npm view @base-ui/react version peerDependencies license dist.unpackedSize --json; npm view @base-ui/react@1.8.0 peerDependenciesMeta dependencies --json; local Cargo.lock inspection.

The navigation-stage build can serve as the pre-Base-UI bundle baseline. Record actual final build size separately.
