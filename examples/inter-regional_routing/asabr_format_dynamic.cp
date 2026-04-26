
# Name aliases are informative (e.g. .._B or .._B_R)

# Nodes that are only part of the home region B
node 0 node0_B
node 1 node1_B
node 2 node2_B

# Node passageways that are also member of region R
node 3 node3_B_R
node 4 node4_B_R

# External nodes member of region G (enode)
enode 5 node5_G
enode 6 node6_G
enode 7 node7_G

# Declare virtual node for node passageways 3 and 4
vnode 8 gateways_to_region_R [ 3 4 ]

# Declare virtual node for external nodes 5, 6, and 7
vnode 9 gateways_to_region_G [ 5 6 7 ]

# Contacts
contact 0 1 60 7260 eto 10000 10
contact 1 2 60 7260 evl 15000 15
contact 2 3 60 7260 evl 20000 20
contact 4 0 60 7260 qd 30000 30

# Passageways contacts
contact 0 5 60 7260 qd 30000 30
contact 6 1 60 7260 qd 30000 30
contact 2 7 60 7260 qd 30000 30
