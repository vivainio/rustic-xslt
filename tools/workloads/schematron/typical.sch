<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt3" defaultPhase="basic">
  <ns prefix="inv" uri="urn:inventory"/>
  <let name="floor" value="1"/>
  <phase id="basic"><active pattern="quantities"/></phase>
  <phase id="alternative"><active pattern="inactive"/></phase>
  <pattern id="quantities">
    <let name="minimum" value="$floor + 1"/>
    <rule context="inv:item">
      <let name="quantity" value="number(@quantity)"/>
      <assert test="$quantity ge $minimum" diagnostics="quantity-info">Quantity for <name/> named <value-of select="@id"/> must be at least two.</assert>
    </rule>
    <rule context="inv:item"><assert test="false()">A later matching rule must be suppressed.</assert></rule>
  </pattern>
  <pattern id="inactive"><rule context="inv:item"><assert test="false()">Inactive pattern must not run.</assert></rule></pattern>
  <diagnostics><diagnostic id="quantity-info">Check the item quantity.</diagnostic></diagnostics>
</schema>
