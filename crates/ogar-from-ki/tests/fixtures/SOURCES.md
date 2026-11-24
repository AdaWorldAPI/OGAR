# Fixture sources

The three `.ki` files are the three examples in the public HIRO 7.0 developer
portal, copied byte for byte from section D of
`docs/handover/HIRO_PUBLIC_DOCS_HARVEST.md` of the Bardioc handover notes. The
Knowledge Item text has no comment form in these examples, so no source line is
written inside the files; the sources are recorded here instead.

| file | public page | what it is |
|---|---|---|
| `tutorial_nothing_to_do.ki` | https://dev-portal.engine.vendor.example/7.0/tutorials/automation-tasks/ | the Knowledge Item of the automation-tasks tutorial (a complete item) |
| `http_request.ki` | https://dev-portal.engine.vendor.example/7.0/documentation/integration/Standard_ActionHandlers/AHconfig/ | "Sample KI for a HTTP request" (a complete item, no `ki` header; it contains tab characters) |
| `ssh_execute_command_line.ki` | https://dev-portal.engine.vendor.example/7.0/documentation/integration/Standard_ActionHandlers/AHconfig/ | the SSH variant: ONE `do` statement, not a complete item. The tests wrap it in a synthetic `on`/`when`/`do` header (labelled synthetic) |

No other Knowledge Item text from HIRO was seen. Everything else under `tests/`
that is item text is hand-written inside the documented subset and labelled
synthetic.
