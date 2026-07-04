"""@generated DO arm — faithful mirror of the `confirmations` controller.
`osm.controllers.confirmations.<is_a>(inp)` free functions (re-exported as
`osm.confirmations`); standalone, not methods on the model. Call:
`osm.controllers.confirmations.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def confirm(inp: Input) -> Output:
    """`confirmations:confirm` — DO arm. Source: ConfirmationsController#confirm."""
    raise NotImplementedError("port ConfirmationsController#confirm")

def confirm_email(inp: Input) -> Output:
    """`confirmations:confirm_email` — DO arm. Source: ConfirmationsController#confirm_email."""
    raise NotImplementedError("port ConfirmationsController#confirm_email")

def confirm_resend(inp: Input) -> Output:
    """`confirmations:confirm_resend` — DO arm. Source: ConfirmationsController#confirm_resend."""
    raise NotImplementedError("port ConfirmationsController#confirm_resend")

def gravatar_status_message(inp: Input) -> Output:
    """`confirmations:gravatar_status_message` — DO arm. Source: ConfirmationsController#gravatar_status_message."""
    raise NotImplementedError("port ConfirmationsController#gravatar_status_message")

