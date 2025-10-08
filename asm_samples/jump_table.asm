
macro jumptable index, tmp, table, n {
        ldpc tmp, table
        add tmp, tmp, index
        ldp tmp, tmp
        jal r0, tmp
}

jumptable! r1, table, 2
nop ; Delay slot
table:
.dw target1
.dw target2
