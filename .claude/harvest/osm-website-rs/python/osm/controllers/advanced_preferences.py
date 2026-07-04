"""@generated DO arm — faithful mirror of the `advanced_preferences` controller.
`osm.controllers.advanced_preferences.<is_a>(inp)` free functions (re-exported as
`osm.advanced_preferences`); standalone, not methods on the model. Call:
`osm.controllers.advanced_preferences.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_preferences(inp: Input) -> Output:
    """`advanced_preferences:update_preferences` — DO arm. Source: Preferences::AdvancedPreferencesController#update_preferences."""
    raise NotImplementedError("port Preferences::AdvancedPreferencesController#update_preferences")

