"""@generated OSM package (ruff → OGAR, Python import shape).
Models: `from osm.models import Node`.
DO arm (faithful): `osm.controllers.nodes.show(inp)`.
DO arm (re-exported): `osm.nodes.show(inp)`.
"""
from . import models as models
from . import controllers as controllers
from .controllers import accounts as accounts
from .controllers import active_lists as active_lists
from .controllers import advanced_preferences as advanced_preferences
from .controllers import api as api
from .controllers import application as application
from .controllers import basic_preferences as basic_preferences
from .controllers import capabilities as capabilities
from .controllers import changeset_comments as changeset_comments
from .controllers import changeset_subscriptions as changeset_subscriptions
from .controllers import changesets as changesets
from .controllers import closes as closes
from .controllers import companies as companies
from .controllers import confirmations as confirmations
from .controllers import dashboards as dashboards
from .controllers import data as data
from .controllers import deletions as deletions
from .controllers import descriptions as descriptions
from .controllers import diary_comments as diary_comments
from .controllers import diary_entries as diary_entries
from .controllers import directions as directions
from .controllers import downloads as downloads
from .controllers import errors as errors
from .controllers import export as export
from .controllers import feature_queries as feature_queries
from .controllers import feeds as feeds
from .controllers import follows as follows
from .controllers import heatmaps as heatmaps
from .controllers import homes as homes
from .controllers import icons as icons
from .controllers import images as images
from .controllers import inboxes as inboxes
from .controllers import issue_comments as issue_comments
from .controllers import issued_blocks as issued_blocks
from .controllers import issues as issues
from .controllers import languages_panes as languages_panes
from .controllers import latlon_queries as latlon_queries
from .controllers import layers_panes as layers_panes
from .controllers import legend_panes as legend_panes
from .controllers import links as links
from .controllers import lists as lists
from .controllers import locations as locations
from .controllers import mailboxes as mailboxes
from .controllers import maps as maps
from .controllers import messages as messages
from .controllers import muted_inboxes as muted_inboxes
from .controllers import mutes as mutes
from .controllers import nodes as nodes
from .controllers import nominatim_queries as nominatim_queries
from .controllers import nominatim_reverse_queries as nominatim_reverse_queries
from .controllers import note_subscriptions as note_subscriptions
from .controllers import notes as notes
from .controllers import notification_preferences as notification_preferences
from .controllers import oauth2_applications as oauth2_applications
from .controllers import old_elements as old_elements
from .controllers import old_nodes as old_nodes
from .controllers import old_relation_members as old_relation_members
from .controllers import old_relations as old_relations
from .controllers import old_ways as old_ways
from .controllers import outboxes as outboxes
from .controllers import passwords as passwords
from .controllers import pd_declarations as pd_declarations
from .controllers import permissions as permissions
from .controllers import pictures as pictures
from .controllers import preferences as preferences
from .controllers import profile_sections as profile_sections
from .controllers import queries as queries
from .controllers import read_marks as read_marks
from .controllers import received_blocks as received_blocks
from .controllers import redactions as redactions
from .controllers import relation_members as relation_members
from .controllers import relations as relations
from .controllers import replies as replies
from .controllers import reporters as reporters
from .controllers import reports as reports
from .controllers import searches as searches
from .controllers import sessions as sessions
from .controllers import share_panes as share_panes
from .controllers import site as site
from .controllers import statuses as statuses
from .controllers import terms as terms
from .controllers import tracepoints as tracepoints
from .controllers import traces as traces
from .controllers import uploads as uploads
from .controllers import user_blocks as user_blocks
from .controllers import user_mutes as user_mutes
from .controllers import user_preferences as user_preferences
from .controllers import user_roles as user_roles
from .controllers import users as users
from .controllers import versions as versions
from .controllers import visibilities as visibilities
from .controllers import ways as ways
from .controllers import webgl_error_panes as webgl_error_panes

CLASS_IDS = {
    "Changeset": 0x0F04,
    "Node": 0x0F01,
    "NodeTag": 0x0F05,
    "Note": 0x0F08,
    "OldNode": 0x0F01,
    "OldNodeTag": 0x0F05,
    "OldRelation": 0x0F03,
    "OldRelationMember": 0x0F06,
    "OldRelationTag": 0x0F05,
    "OldWay": 0x0F02,
    "OldWayNode": 0x0F07,
    "OldWayTag": 0x0F05,
    "Relation": 0x0F03,
    "RelationMember": 0x0F06,
    "RelationTag": 0x0F05,
    "Trace": 0x0F09,
    "User": 0x0F0A,
    "Way": 0x0F02,
    "WayNode": 0x0F07,
    "WayTag": 0x0F05,
}
