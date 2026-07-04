"""@generated DO arm — faithful mirror of the `note_subscriptions` controller.
`osm.controllers.note_subscriptions.<is_a>(inp)` free functions (re-exported as
`osm.note_subscriptions`); standalone, not methods on the model. Call:
`osm.controllers.note_subscriptions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`note_subscriptions:create` — DO arm. Source: Api::NoteSubscriptionsController#create."""
    raise NotImplementedError("port Api::NoteSubscriptionsController#create")

def delete(inp: Input) -> Output:
    """`note_subscriptions:delete` — DO arm. Source: Api::NoteSubscriptionsController#destroy."""
    raise NotImplementedError("port Api::NoteSubscriptionsController#destroy")

