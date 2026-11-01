<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt3">
  <ns prefix="inv" uri="urn:inventory"/>
  <pattern id="inventory">
    <rule context="inv:item">
      <assert test="@id">An item needs an id.</assert>
      <report test="@discontinued = 'yes'">Discontinued item.</report>
    </rule>
  </pattern>
</schema>
