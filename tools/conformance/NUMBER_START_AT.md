# xsl:number start-at implementation requirements

Reference: https://www.w3.org/TR/xslt-30/#start-at

The effective AVT value matches -?[0-9]+(\s+-?[0-9]+)*. A leading plus sign is
not permitted. The list is nonempty and uses XML whitespace. Signed values are
required, including negative offsets and negative zero.

For each place-marker component V[i], add S[i]-1 if S has that component,
otherwise add S[last()]-1. Ignore excess offsets. Apply this rebasing to both
supplied value sequences and computed single/multiple/any node numbering before
formatting. Preserve empty place markers and existing numbering conversion rules.

Implemented in increments 208-210: NumberSpec retains the start-at AVT, static
scope validation checks it, and runtime rebasing uses signed arbitrary-precision
integers. Invalid literal and dynamic lists report XTSE0020 and XTDE0030.
Supplied integers and decimals round through the numeric implementation rather
than narrowing through floating point and u64. Decimal picture width, digit
families and grouping are retained for large results. Non-decimal formats above
the existing u64 formatter range currently fall back to decimal digits.
Record streaming continues to reject xsl:number as a whole during static safety
validation, before reading input.

Pinned fixtures number-0102 through number-0109 cover zero, dynamic offsets,
multiple components, repeated offsets, signed offsets and invalid syntax.
number-0111 exercises large values and passes in the committed regression gate.
