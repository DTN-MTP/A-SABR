
# Node entry with no management: node <id> <name>
# Home region (region X)
node 0 node0_X
node 1 node1_X
node 2 node2_X
# Nodes that are also part of region Y
node 3 node3_X_Y
node 4 node4_X_Y

# External nodes of region Z
enode 5 node5_Z
enode 6 node6_Z
enode 7 node7_Z

# Nodes 3, 4, and 5 are node passageways
vnode 8 gateways_to_region_Y [ 3 4 ]

# Nodes 6, and 7, are external nodes, contacts with those nodes are contact passageways
vnode 9 gateways_to_region_Z [ 5 6 7 ]

# Dynamic parsing for contacts a marker should appear before the manager tokens:
# contact <from> <to> <start> <end> <marker> ...
contact 0 1 60 7260 eto 10000 10
contact 1 2 60 7260 evl 15000 15
contact 2 3 60 7260 evl 20000 20
contact 4 0 60 7260 qd 30000 30
contact 0 5 60 7260 seg rate 60 3660 10000 rate 3660 7260 15000 delay 60 7260 12
contact 6 1 60 7260 qd 30000 30
contact 2 7 60 7260 qd 30000 30
