# xsl:number start-at implementation requirements

Reference: https://www.w3.org/TR/xslt-30/#start-at

The effective AVT value matches -?[0-9]+(\s+-?[0-9]+)*. A leading plus sign is
not permitted. The list is nonempty and uses XML whitespace. Signed values are
required, including negative offsets and negative zero.

For each place-marker component V[i], add S[i]-1 if S has that component,
otherwise add S[last()]-1. Ignore excess offsets. Apply this rebasing to both
supplied value sequences and computed single/multiple/any node numbering before
formatting. Preserve empty place markers and existing numbering conversion rules.

Current gaps: NumberSpec has no start-at AVT, the compiler explicitly rejects the
attribute, runtime number_text builds Vec<u64>, and numbering::format_list takes
unsigned values. Signed rebasing and signed formatting must be handled together;
clamping negative results or wrapping unsigned arithmetic would be incorrect.
Static scope validation and record-streaming expression traversal must include
the new AVT. Invalid literal and dynamic syntax need the correct XSLT diagnostic.

Pinned fixtures number-0102 through number-0109 cover zero, dynamic offsets,
multiple components, repeated offsets, signed offsets and invalid syntax.
number-0111 also exercises large values, which exposes the separate existing
floating-point/u64 narrowing in supplied-number conversion.
