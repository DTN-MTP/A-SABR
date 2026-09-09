node 0 Earth
node 1 Mars

# Static contact parsed directly by PolySegManager
# Format : contact <from> <to> <start> <end> delay [<start> <end> <val>] poly [<c0> <c1> <c2>] <offset>
# Here: flow rate = 10 + 3*t^2, network delay = 4
contact 0 1 0 100 delay [0 100 4] poly [10, 0, 3] 0