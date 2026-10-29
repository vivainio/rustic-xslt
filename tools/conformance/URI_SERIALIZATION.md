# URI attribute serialization implementation requirements

Reference: W3C XSLT and XQuery Serialization 3.0, URI escaping and Appendix D:
https://www.w3.org/TR/xslt-xquery-serialization-30/#uri-escaping
https://www.w3.org/TR/xslt-xquery-serialization-30/#list-of-uri-attributes

The serializer must apply the following sequence when URI escaping is enabled:
NFC normalization, escape-html-uri percent encoding, then the selected XML/HTML
attribute escaping. Character maps must not apply to these URI values. Relative
URIs must remain relative. XML output is unaffected by this parameter.

URI recognition uses the element and attribute combination, with HTML/XHTML
namespace rules and unnamespaced attributes. The Appendix D table is:

| Attribute | Elements |
| --- | --- |
| action | form |
| archive, classid, data | object |
| background | body |
| cite | blockquote, del, ins, q |
| codebase | applet, object |
| datasrc | button, div, input, object, select, span, table, textarea |
| for | script |
| formaction | button, input |
| href | a, area, base, link |
| icon | command |
| longdesc | frame, iframe, img |
| manifest | html |
| name | a |
| poster | video |
| profile | head |
| src | audio, embed, frame, iframe, img, input, script, source, track, video |
| usemap | img, input, object |
| value | input |

Implementation must carry escape-uri-attributes through output declarations,
named output definitions, explicit result-document AVTs and nested output
captures. Tests must distinguish URI attributes from ordinary and namespaced
attributes, exercise NFC before encoding, retain ASCII URI punctuation, and
cover both enabled and disabled character-map interaction.

The existing result-document fixtures 0263 through 0268 and 1204 cover explicit
URI escaping parameters. The serializer currently has no URI-escaping state;
HTML attributes are escaped directly in Output::flush_start.
