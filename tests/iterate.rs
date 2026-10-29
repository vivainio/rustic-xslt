use rust_xslt::Stylesheet;
fn run(body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform(source)
}
const SOURCE: &str = "<r><n>1</n><n>2</n><n>3</n><n>4</n></r>";
#[test]
fn iterate_threads_parameters_between_iterations() {
    let running_total = r#"<xsl:iterate select="r/n"><xsl:param name="total" select="0"/><xsl:variable name="new" select="$total + ."/><xsl:value-of select="$new"/>,<xsl:next-iteration><xsl:with-param name="total" select="$new"/></xsl:next-iteration></xsl:iterate>"#;
    assert_eq!(run(running_total, SOURCE).unwrap(), "1,3,6,10,");
    let completion = r#"<xsl:iterate select="r/n"><xsl:param name="total" select="0"/><xsl:on-completion>total=<xsl:value-of select="$total"/></xsl:on-completion><xsl:next-iteration><xsl:with-param name="total" select="$total + ."/></xsl:next-iteration></xsl:iterate>"#;
    assert_eq!(run(completion, SOURCE).unwrap(), "total=10");
    let completion_select = r#"<xsl:iterate select="r/n"><xsl:param name="count" select="0" as="xs:integer" xmlns:xs="http://www.w3.org/2001/XMLSchema"/><xsl:on-completion select="$count"/><xsl:next-iteration><xsl:with-param name="count" select="$count + 1"/></xsl:next-iteration></xsl:iterate>"#;
    assert_eq!(run(completion_select, SOURCE).unwrap(), "4");
}
#[test]
fn position_and_size_follow_the_iterated_sequence() {
    let body = r#"<xsl:iterate select="r/n"><xsl:value-of select="position()"/>/<xsl:value-of select="last()"/>;</xsl:iterate>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "1/4;2/4;3/4;4/4;");
    assert_eq!(run(body, "<r/>").unwrap(), "");
}
#[test]
fn break_ends_the_loop_and_skips_on_completion() {
    let body = r#"<xsl:iterate select="r/n"><xsl:on-completion>done</xsl:on-completion><xsl:choose><xsl:when test=". = 3"><xsl:break>stop at <xsl:value-of select="."/></xsl:break></xsl:when><xsl:otherwise><xsl:value-of select="."/>,<xsl:next-iteration/></xsl:otherwise></xsl:choose></xsl:iterate>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "1,2,stop at 3");
    let select_break = r#"<xsl:iterate select="r/n"><xsl:break select="'first'"/></xsl:iterate>"#;
    assert_eq!(run(select_break, SOURCE).unwrap(), "first");
}
#[test]
fn unchanged_parameters_carry_over_and_errors_are_reported() {
    let carry = r#"<xsl:iterate select="r/n"><xsl:param name="a" select="'x'"/><xsl:param name="b" select="0"/><xsl:value-of select="$a, $b"/>;<xsl:next-iteration><xsl:with-param name="b" select="$b + 1"/></xsl:next-iteration></xsl:iterate>"#;
    assert_eq!(run(carry, SOURCE).unwrap(), "x 0;x 1;x 2;x 3;");
    let unknown = r#"<xsl:iterate select="r/n"><xsl:next-iteration><xsl:with-param name="nope" select="1"/></xsl:next-iteration></xsl:iterate>"#;
    assert_eq!(run(unknown, SOURCE).unwrap_err().code, "XTSE3130");
    let required =
        r#"<xsl:iterate select="r/n"><xsl:param name="p" required="yes"/></xsl:iterate>"#;
    assert!(run(required, SOURCE).is_err());
    assert!(run(r#"<xsl:next-iteration/>"#, SOURCE).is_err());
}
