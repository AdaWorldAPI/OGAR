"""@generated DO arm — faithful mirror of the `mailboxes` controller.
`osm.controllers.mailboxes.<is_a>(inp)` free functions (re-exported as
`osm.mailboxes`); standalone, not methods on the model. Call:
`osm.controllers.mailboxes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show_messages(inp: Input) -> Output:
    """`mailboxes:show_messages` — DO arm. Source: Api::Messages::MailboxesController#show_messages."""
    raise NotImplementedError("port Api::Messages::MailboxesController#show_messages")

