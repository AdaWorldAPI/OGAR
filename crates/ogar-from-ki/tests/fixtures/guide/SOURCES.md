# Guide fixtures

The ten `.ki` files are the ten complete Knowledge Items printed in the HIRO
User Guides 7.0, section "KI Syntax":
https://user-guides.engine.datagroup.de/7.0/ki-syntax/

Each file is the content of one code block, copied byte for byte. Nothing is
fixed, reformatted or completed, so any defect in the guide is a defect in the
fixture. The text was taken from the harvested page files
(`docs/handover/hiro_ki_guide/pages/` of the Bardioc handover notes) and each
file was compared with the line range of its page: all ten are identical.

| file | guide page | lines of the harvested page text |
|---|---|---|
| `full_example.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("1.Full Example") | 16-47 |
| `retry_action.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("2.Retry action KI") | 51-74 |
| `add_comment_valuemation.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("3.Add Comment to ticketing System") | 78-108 |
| `fallback_unresolved.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("4. Fallback KI") | 112-144 |
| `checking_spelling.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("5. Checking Spelling") | 148-186 |
| `working_with_dates.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("6. Working with dates") | 190-216 |
| `querying_graph.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/example/ ("7. Querying Graph") | 220-247 |
| `lists_ki1_delete_files_on.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/AllAboutLists/ ("KI 1") | 33-46 |
| `lists_ki2_list_delete_files_on.ki` | https://user-guides.engine.datagroup.de/7.0/ki-syntax/definitions/AllAboutLists/ ("KI 2") | 48-61 |
| `rl_hn3_r1.ki` | https://user-guides.engine.datagroup.de/7.0/guides/rl-user-guide/ ("HN3_R1") | 78-96 |

The three portal fixtures one directory up come from a different source (the
developer portal) and are described in `../SOURCES.md`.
