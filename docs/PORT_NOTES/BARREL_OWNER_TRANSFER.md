# Resting barrel struck by an owned article

`it_80270E30` reads the striking item's **fighter owner** into xCF0
(retail 0x80271160..0x80271194); that field does not name the item itself.
`it_8027236C` transfers that fighter and team, then `it_8027B1F4` resets
attack ID to 1 and instance to 0 (0x8027B214..0x8027B224).

The enabled slice is an initially unowned resting barrel hit by an article
with a primary fighter owner. The strongest hit retains its owner's
secondary identity so unsupported follower owners are detected. Previously
owned recipients, nonresting transfers and articles without a primary
fighter owner remain guarded. Direct fighter transfers and reflection
retain their previous semantics; reflection preserves attack identity,
whereas this transfer clears it.

Existing full retail captures prove Fox Illusion hitting a resting barrel:
`corpus_v3_bf_items_sword_e44c79d87_p2` (252 ticks, contact 251) and
`corpus_v3_fd_items_sword_e3b85eba6_p1` (282 ticks, contact 281). On contact
the barrel enters rolling, belongs to Fox's fighter GObj/team, and creates a
15-damage hitbox with attack ID1/instance0. The article's own attack ID19 is
not inherited. Both windows end at contact; they do not independently prove
all later transferred-barrel damage attribution. Existing longer fighter-hit
barrel recordings cover the common rolling continuation.
