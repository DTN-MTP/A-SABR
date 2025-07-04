
# Node entry with no management: node <id> <name>
node 0 node1
node 1 node2
node 2 node3
node 3 node4
node 4 node5
node 5 node6

# Dynamic parsing for contacts a marker should appear before the manager tokens:
# contact <from> <to> <start> <end> <marker> ...
contact 0 1 60 7260 eto 10000 10
contact 1 2 60 7260 evl 15000 15
contact 2 3 60 7260 evl 20000 20
contact 3 4 60 7260 qd 25000 25
contact 4 5 60 7260 qd 30000 30

# A segmented contact after the marker, the parser expects a sequence of delay/rate intervals:
# delay <start> <end> <delay>
# rate <start> <end> <rate>
contact 0 5 60 7260 seg rate 60 3660 10000 rate 3660 7260 15000 delay 60 7260 12

# A-SABR format does not care of whitespaces, including new lines:
contact 1 4 60 7260 seg
delay 60 3660 10
delay 3660 7260 15
rate 60 7260 10000
