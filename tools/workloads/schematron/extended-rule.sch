<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt3">
  <ns prefix="inv" uri="urn:inventory"/>
  <pattern>
    <rule id="required-id" abstract="true"><assert test="@id">Extended rule requires an id.</assert></rule>
    <rule context="inv:item"><extends rule="required-id"/></rule>
  </pattern>
</schema>
