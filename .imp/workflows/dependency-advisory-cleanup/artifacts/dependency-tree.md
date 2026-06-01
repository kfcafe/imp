# Dependency advisory triage

Source:  (Scanning dir .
Warning: plugin transitivedependency/pomxml can be risky when run on untrusted artifacts. Please ensure you trust the source code and artifacts before proceeding.
Starting filesystem walk for root: /
Scanned /Users/asher/imp/Cargo.lock file and found 689 packages
End status: 348 dirs visited, 5746 inodes visited, 1 Extract calls, 154.706916ms elapsed, 154.708ms wall time

Total 5 packages affected by 5 known vulnerabilities (0 Critical, 0 High, 0 Medium, 1 Low, 4 Unknown) from 1 ecosystem.
1 vulnerability can be fixed.

+-------------------------------------+------+-----------+-----------+---------+---------------+------------+
| OSV URL                             | CVSS | ECOSYSTEM | PACKAGE   | VERSION | FIXED VERSION | SOURCE     |
+-------------------------------------+------+-----------+-----------+---------+---------------+------------+
| https://osv.dev/RUSTSEC-2025-0141   |      | crates.io | bincode   | 1.3.3   | --            | Cargo.lock |
| https://osv.dev/RUSTSEC-2025-0057   |      | crates.io | fxhash    | 0.2.1   | --            | Cargo.lock |
| https://osv.dev/RUSTSEC-2026-0002   | 2.7  | crates.io | lru       | 0.12.5  | 0.16.3        | Cargo.lock |
| https://osv.dev/GHSA-rhfx-m35p-ff5j |      |           |           |         |               |            |
| https://osv.dev/RUSTSEC-2024-0436   |      | crates.io | paste     | 1.0.15  | --            | Cargo.lock |
| https://osv.dev/RUSTSEC-2024-0320   |      | crates.io | yaml-rust | 0.4.5   | --            | Cargo.lock |
+-------------------------------------+------+-----------+-----------+---------+---------------+------------+).

Findings:
- RUSTSEC-2025-0141: bincode 1.3.3, no fixed version.
- RUSTSEC-2025-0057: fxhash 0.2.1, no fixed version.
- RUSTSEC-2026-0002 / GHSA-rhfx-m35p-ff5j: lru 0.12.5, fixed in 0.16.3.
- RUSTSEC-2024-0436: paste 1.0.15, unmaintained/no fixed version.
- RUSTSEC-2024-0320: yaml-rust 0.4.5, no fixed version.

## Reverse dependency trees

### bincode
```
bincode v1.3.3
└── syntect v5.3.0
    └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui)
        └── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli)
            └── imp-install v0.3.0 (/Users/asher/imp)
```

### fxhash
```
fxhash v0.2.1
└── selectors v0.25.0
    └── scraper v0.18.1
        └── readability-rust v0.1.0
            └── imp-core v0.3.0 (/Users/asher/imp/crates/imp-core)
                ├── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli)
                │   └── imp-install v0.3.0 (/Users/asher/imp)
                ├── imp-lua v0.3.0 (/Users/asher/imp/crates/imp-lua)
                │   ├── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli) (*)
                │   └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui)
                │       └── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli) (*)
                └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui) (*)
```

### lru
```
lru v0.12.5
└── ratatui v0.29.0
    └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui)
        └── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli)
            └── imp-install v0.3.0 (/Users/asher/imp)
```

### paste
```
paste v1.0.15 (proc-macro)
└── ratatui v0.29.0
    └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui)
        └── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli)
            └── imp-install v0.3.0 (/Users/asher/imp)
```

### yaml-rust
```
yaml-rust v0.4.5
└── syntect v5.3.0
    └── imp-tui v0.3.0 (/Users/asher/imp/crates/imp-tui)
        └── imp-cli v0.3.0 (/Users/asher/imp/crates/imp-cli)
            └── imp-install v0.3.0 (/Users/asher/imp)
```
