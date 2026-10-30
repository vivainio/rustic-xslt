use rust_xslt::Stylesheet;
fn run(declarations: &str, select: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xsl:output method="text"/>{declarations}<xsl:template match="/"><xsl:sequence select="{select}"/></xsl:template></xsl:stylesheet>"#))?
        .transform("<r><n>1</n><n>2</n><n>3</n></r>")
}
#[test]
fn functions_take_typed_parameters_and_return_sequences() {
    let double = r#"<xsl:function name="f:double" as="xs:integer"><xsl:param name="x" as="xs:integer"/><xsl:sequence select="$x * 2"/></xsl:function>"#;
    assert_eq!(run(double, "f:double(21)").unwrap(), "42");
    assert_eq!(run(double, "f:double(f:double(3))").unwrap(), "12");
    assert_eq!(
        run(
            double,
            "string-join(for $i in 1 to 3 return string(f:double($i)), ',')"
        )
        .unwrap(),
        "2,4,6"
    );
    assert_eq!(run(double, "f:double(r/n[2])").unwrap(), "4");
    assert!(run(double, "f:double('a')").is_err());
    assert!(run(double, "f:double(1, 2)").is_err());
    assert!(run(double, "f:double(())").is_err());
}
#[test]
fn function_bodies_see_locals_params_and_global_variables() {
    let declarations = r#"<xsl:variable name="base" select="100"/>
        <xsl:function name="f:add"><xsl:param name="a"/><xsl:param name="b"/>
          <xsl:variable name="sum" select="$a + $b"/>
          <xsl:sequence select="$sum + $base"/></xsl:function>
        <xsl:function name="f:count"><xsl:param name="nodes"/><xsl:sequence select="count($nodes)"/></xsl:function>
        <xsl:function name="f:fact" as="xs:integer"><xsl:param name="n" as="xs:integer"/>
          <xsl:sequence select="if ($n le 1) then 1 else $n * f:fact($n - 1)"/></xsl:function>"#;
    assert_eq!(run(declarations, "f:add(1, 2)").unwrap(), "103");
    assert_eq!(run(declarations, "f:count(r/n)").unwrap(), "3");
    assert_eq!(run(declarations, "f:fact(10)").unwrap(), "3628800");
    // A caller's local variable does not leak into the function body.
    let shadow = r#"<xsl:variable name="v" select="'global'"/><xsl:function name="f:v"><xsl:sequence select="$v"/></xsl:function>"#;
    assert_eq!(
        run(shadow, "let $v := 'local' return f:v()").unwrap(),
        "global"
    );
}
#[test]
fn function_declaration_errors_are_static() {
    let dup = r#"<xsl:function name="f:a"><xsl:sequence select="1"/></xsl:function><xsl:function name="f:a"><xsl:sequence select="2"/></xsl:function>"#;
    assert_eq!(run(dup, "1").unwrap_err().code, "XTSE0770");
    let overloaded = r#"<xsl:function name="f:a"><xsl:sequence select="1"/></xsl:function><xsl:function name="f:a"><xsl:param name="x"/><xsl:sequence select="2"/></xsl:function>"#;
    assert_eq!(run(overloaded, "f:a() + f:a(0)").unwrap(), "3");
    let unqualified = r#"<xsl:function name="a"><xsl:sequence select="1"/></xsl:function>"#;
    assert_eq!(run(unqualified, "1").unwrap_err().code, "XTSE0080");
    let unbound = r#"<xsl:function name="f:a"><xsl:sequence select="$nope"/></xsl:function>"#;
    assert_eq!(run(unbound, "1").unwrap_err().code, "XPST0008");
    assert_eq!(run("", "f:missing()").unwrap_err().code, "XPST0017");
}

#[test]
fn pure_function_constructors_support_branches_and_scoped_sequences() {
    let declarations = r#"<xsl:function name="f:values"><xsl:param name="n"/><xsl:sequence select="0"/><xsl:variable name="x" select="$n * 2"/><xsl:if test="$n gt 1"><xsl:variable name="y" select="$x + 1"/><xsl:sequence select="$y"/></xsl:if><xsl:choose><xsl:when test="$n = 1"><xsl:sequence select="11, 12"/></xsl:when><xsl:when test="$n = 2"><xsl:sequence select="$x"/></xsl:when><xsl:otherwise><xsl:sequence select="99"/></xsl:otherwise></xsl:choose><xsl:sequence select="$x"/></xsl:function>"#;
    assert_eq!(
        run(declarations, "string-join(f:values(1)!string(.), ',')").unwrap(),
        "0,11,12,2"
    );
    assert_eq!(
        run(declarations, "string-join(f:values(2)!string(.), ',')").unwrap(),
        "0,5,4,4"
    );
    assert_eq!(
        run(declarations, "string-join(f:values(3)!string(.), ',')").unwrap(),
        "0,7,99,6"
    );
    let invalid = r#"<xsl:function name="f:bad"><xsl:if test="false()"><xsl:variable name="private" select="1"/></xsl:if><xsl:sequence select="$private"/></xsl:function>"#;
    assert_eq!(run(invalid, "1").unwrap_err().code, "XPST0008");
}
