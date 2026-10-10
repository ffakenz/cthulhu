# Design records

How the engine treats its input: the solution-space decisions summarised in
[docs/overview.md](../overview.md). The architecture they comply with is in
[docs/adr/](../adr/README.md).

The conventions are the architecture log's: one numbered record per decision,
`NNNN-kebab-case-title.md`, copied from `docs/adr/template.md`. A reversed record is superseded,
never edited; its status becomes `Superseded by M`, where M is a design record's number. Only this
index and those status lines carry a number.

## Index

- [1. Exact decimal amounts with four places](0001-exact-decimal-amounts-with-four-places.md)
- [2. Rows parsed into commands at the boundary](0002-rows-parsed-into-commands-at-the-boundary.md)
- [3. Tx ids consumed on first sight](0003-tx-ids-consumed-on-first-sight.md)
- [4. Accounts opened by the first command](0004-accounts-opened-by-the-first-command.md)
- [5. A per-deposit dispute lifecycle](0005-a-per-deposit-dispute-lifecycle.md)
- [6. Client commands refused on locked accounts](0006-client-commands-refused-on-locked-accounts.md)
- [7. The command-line contract](0007-the-command-line-contract.md)
