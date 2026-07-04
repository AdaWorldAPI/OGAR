"""@generated DO arm — faithful mirror of the `changeset_subscriptions` controller.
`osm.controllers.changeset_subscriptions.<is_a>(inp)` free functions (re-exported as
`osm.changeset_subscriptions`); standalone, not methods on the model. Call:
`osm.controllers.changeset_subscriptions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`changeset_subscriptions:create` — DO arm. Sources (canonical tile): Api::ChangesetSubscriptionsController#create, ChangesetSubscriptionsController#create."""
    raise NotImplementedError("port Api::ChangesetSubscriptionsController#create")

def delete(inp: Input) -> Output:
    """`changeset_subscriptions:delete` — DO arm. Sources (canonical tile): Api::ChangesetSubscriptionsController#destroy, ChangesetSubscriptionsController#destroy."""
    raise NotImplementedError("port Api::ChangesetSubscriptionsController#destroy")

def show(inp: Input) -> Output:
    """`changeset_subscriptions:show` — DO arm. Source: ChangesetSubscriptionsController#show."""
    raise NotImplementedError("port ChangesetSubscriptionsController#show")

