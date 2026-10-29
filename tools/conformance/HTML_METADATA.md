# HTML and XHTML content-type serialization requirements

Reference: W3C XSLT and XQuery Serialization 3.0, sections 6.1.14 and 7.4.13:
https://www.w3.org/TR/xslt-xquery-serialization-30/#XHTML_INCLUDE_CONTENT_TYPE
https://www.w3.org/TR/xslt-xquery-serialization-30/#HTML_INCLUDE_CONTENT_TYPE

When include-content-type is enabled and a recognized HTML head exists, insert
one generated meta element as its first child element. Its content must use the
configured media-type and actual encoding. HTML emits a void meta tag; XHTML
emits an XHTML meta element with XML empty-element syntax.

After insertion, discard existing direct meta children of that head when their
unnamespaced http-equiv value equals Content-Type after trimming leading and
trailing spaces and comparing without regard to case. Preserve unrelated meta
elements. Disabled insertion preserves existing elements. XML and text output
ignore this parameter.

Implementation must carry include-content-type, media-type and actual encoding
through unnamed/named output definitions and result-document AVTs. Nested try
and populated captures must inherit the surrounding serialization configuration
without injecting duplicate metadata when captured strings are spliced back.
Output limits must include generated metadata. Namespace and case rules must
follow the chosen HTML/XHTML method rather than matching every local head name.

Current architecture: Output delays start tags until attributes are known.
flush_start_with can inject the generated meta immediately after a head start
tag. Existing meta suppression needs node-aware state through the corresponding
end operation, including copied nodes, so replacement cannot be implemented as
an unrestricted serialized-string substitution.

Relevant result-document cases include 0209, 0210, 0214, 0215, 0223-0226, 0701
and 1205. Default-method detection is a separate requirement from explicit
HTML/XHTML metadata generation.
