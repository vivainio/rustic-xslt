<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt3">
  <ns prefix="inv" uri="urn:inventory"/>
  <pattern id="abstract-check" abstract="true">
    <rule context="$context"><assert test="$predicate">Abstract rule requires an id.</assert></rule>
  </pattern>
  <pattern id="instantiated" is-a="abstract-check">
    <param name="context" value="inv:item"/>
    <param name="predicate" value="@id"/>
  </pattern>
</schema>
