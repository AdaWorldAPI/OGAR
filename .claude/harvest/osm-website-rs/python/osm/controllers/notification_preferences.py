"""@generated DO arm — faithful mirror of the `notification_preferences` controller.
`osm.controllers.notification_preferences.<is_a>(inp)` free functions (re-exported as
`osm.notification_preferences`); standalone, not methods on the model. Call:
`osm.controllers.notification_preferences.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_preferences(inp: Input) -> Output:
    """`notification_preferences:update_preferences` — DO arm. Source: Preferences::NotificationPreferencesController#update_preferences."""
    raise NotImplementedError("port Preferences::NotificationPreferencesController#update_preferences")

