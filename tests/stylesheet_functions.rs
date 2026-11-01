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
    assert_eq!(run(unqualified, "1").unwrap_err().code, "XTSE0740");
    let unbound = r#"<xsl:function name="f:a"><xsl:sequence select="$nope"/></xsl:function>"#;
    assert_eq!(run(unbound, "1").unwrap_err().code, "XPST0008");
    assert_eq!(run("", "f:missing()").unwrap_err().code, "XPST0017");
}

#[test]
fn stylesheet_function_calls_are_resolved_statically_in_all_expression_contexts() {
    for body in [
        r#"<xsl:template match="*[f:missing(.)]"/>"#,
        r#"<xsl:template match="/"><xsl:if test="false()"><xsl:sequence select="f:missing()"/></xsl:if></xsl:template>"#,
        r#"<xsl:variable name="v" select="f:missing#1"/><xsl:template match="/"/>"#,
        r#"<xsl:key name="k" match="*" use="f:missing(.)"/><xsl:template match="/"/>"#,
        r#"<xsl:function name="f:test"><xsl:sequence select="f:missing()"/></xsl:function><xsl:template match="/"/>"#,
    ] {
        let stylesheet = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f">{body}</xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile(&stylesheet).unwrap_err().code,
            "XPST0017",
            "{body}"
        );
    }
    let forward = r#"<xsl:function name="f:first"><xsl:sequence select="f:later()"/></xsl:function><xsl:function name="f:later" visibility="private"><xsl:sequence select="42"/></xsl:function>"#;
    assert_eq!(run(forward, "f:first()").unwrap(), "42");
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

#[test]
fn typed_local_bindings_in_function_bodies_use_function_conversion() {
    let valid = r#"<xsl:function name="f:local"><xsl:param name="n"/><xsl:variable name="x" as="xs:integer" select="$n"/><xsl:sequence select="$x + 1"/></xsl:function>"#;
    assert_eq!(run(valid, "f:local(r/n[1])").unwrap(), "2");
    assert_eq!(run(valid, "f:local('wrong')").unwrap_err().code, "XTTE0570");
    let branch = r#"<xsl:function name="f:branch"><xsl:param name="n"/><xsl:if test="$n"><xsl:variable name="x" as="xs:integer" select="'bad'"/><xsl:sequence select="$x"/></xsl:if></xsl:function>"#;
    assert_eq!(run(branch, "f:branch(false())").unwrap(), "");
    assert_eq!(
        run(branch, "f:branch(true())").unwrap_err().code,
        "XTTE0570"
    );
}

#[test]
fn pure_function_for_each_preserves_focus_order_and_sorting() {
    let declaration = r#"<xsl:function name="f:loop"><xsl:param name="items"/><xsl:for-each select="$items"><xsl:sort select="." data-type="number" order="descending"/><xsl:variable name="value" select="."/><xsl:sequence select="concat($value, ':', position(), '/', last(), ':', current())"/></xsl:for-each></xsl:function>"#;
    assert_eq!(
        run(declaration, "string-join(f:loop((2,1,3)), ',')").unwrap(),
        "3:1/3:3,2:2/3:2,1:3/3:1"
    );
    assert_eq!(run(declaration, "count(f:loop(()))").unwrap(), "0");
    let nested = r#"<xsl:function name="f:nested"><xsl:for-each select="(1,2)"><xsl:variable name="outer" select="."/><xsl:for-each select="(3,4)"><xsl:sequence select="concat($outer, current(), position(), last())"/></xsl:for-each></xsl:for-each></xsl:function>"#;
    assert_eq!(
        run(nested, "string-join(f:nested(), ',')").unwrap(),
        "1312,1422,2312,2422"
    );
}

#[test]
fn function_declaration_boolean_attributes_are_checked() {
    for attributes in [
        "override=' '",
        "override-extension-function='Yes'",
        "new-each-time='Yes'",
        "cache='maybe'",
        "override='no' override-extension-function='1'",
    ] {
        let declaration = format!(
            r#"<xsl:function name="f:test" {attributes}><xsl:sequence select="1"/></xsl:function>"#
        );
        assert_eq!(
            run(&declaration, "f:test()").unwrap_err().code,
            "XTSE0020",
            "{attributes}"
        );
    }
    for attributes in [
        "override='yes' override-extension-function='1'",
        "override='false' override-extension-function='0'",
        "new-each-time='maybe' cache='yes'",
    ] {
        let declaration = format!(
            r#"<xsl:function name="f:test" {attributes}><xsl:sequence select="1"/></xsl:function>"#
        );
        assert_eq!(run(&declaration, "f:test()").unwrap(), "1");
    }
}

#[test]
fn function_parameters_are_required_and_have_no_defaults() {
    for setting in [
        "required='yes'",
        "required='1' tunnel='no'",
        "required='true' static='false'",
    ] {
        let declaration = format!(
            r#"<xsl:function name="f:id"><xsl:param name="p" {setting}/><xsl:sequence select="$p"/></xsl:function>"#
        );
        assert_eq!(run(&declaration, "f:id(42)").unwrap(), "42");
    }
    for setting in [
        "required='no'",
        "required='maybe'",
        "tunnel='yes'",
        "static='yes'",
    ] {
        let declaration = format!(
            r#"<xsl:function name="f:id"><xsl:param name="p" {setting}/><xsl:sequence select="$p"/></xsl:function>"#
        );
        assert_eq!(run(&declaration, "1").unwrap_err().code, "XTSE0020");
    }
    for param in [
        r#"<xsl:param name="p" select="1"/>"#,
        r#"<xsl:param name="p"><xsl:sequence select="1"/></xsl:param>"#,
        r#"<xsl:param name="p" xml:space="preserve"> </xsl:param>"#,
    ] {
        let declaration = format!(
            r#"<xsl:function name="f:id">{param}<xsl:sequence select="$p"/></xsl:function>"#
        );
        assert_eq!(run(&declaration, "1").unwrap_err().code, "XTSE0760");
    }
    assert_eq!(
        run(
            r#"<xsl:function name="Q{}no-namespace"><xsl:sequence select="1"/></xsl:function>"#,
            "1"
        )
        .unwrap_err()
        .code,
        "XTSE0740"
    );
}

#[test]
fn functions_and_parameters_must_appear_in_their_declaration_positions() {
    let nested = r#"<xsl:template match="/"><xsl:function name="f:inner"><xsl:sequence select="1"/></xsl:function></xsl:template>"#;
    assert_eq!(run(nested, "1").unwrap_err().code, "XTSE0010");
    let late = r#"<xsl:function name="f:late"><xsl:sequence select="1"/><xsl:param name="p"/></xsl:function>"#;
    assert_eq!(run(late, "1").unwrap_err().code, "XTSE0010");
}

#[test]
fn obsolete_sequence_type_attribute_is_a_static_error() {
    let declaration =
        r#"<xsl:function name="f:bad"><xsl:sequence as="xs:integer" select="1"/></xsl:function>"#;
    assert_eq!(run(declaration, "1").unwrap_err().code, "XTSE0090");
    let stylesheet = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><xsl:sequence as="xs:integer" select="1"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(stylesheet).unwrap_err().code,
        "XTSE0090"
    );
}

#[test]
fn typed_sequence_constructor_bindings_in_functions_preserve_node_identity_and_order() {
    let declaration = r#"<xsl:function name="f:sorted"><xsl:param name="nodes" as="element()*"/><xsl:variable name="sorted" as="element()*"><xsl:for-each select="$nodes"><xsl:sort select="." data-type="number" order="descending"/><xsl:sequence select="."/></xsl:for-each></xsl:variable><xsl:sequence select="$sorted"/></xsl:function>"#;
    assert_eq!(
        run(declaration, "string-join(f:sorted(r/n) ! string(.), ',')").unwrap(),
        "3,2,1"
    );
    assert_eq!(
        run(declaration, "f:sorted(r/n)[1] is r/n[3]").unwrap(),
        "true"
    );
    assert_eq!(run(declaration, "count(f:sorted(()))").unwrap(), "0");
    let wrong = r#"<xsl:function name="f:bad"><xsl:variable name="nodes" as="element()*"><xsl:for-each select="(1, 2)"><xsl:sequence select="."/></xsl:for-each></xsl:variable><xsl:sequence select="$nodes"/></xsl:function>"#;
    assert_eq!(run(wrong, "f:bad()").unwrap_err().code, "XTTE0570");
}

#[test]
fn function_constructors_return_real_nodes_and_share_the_runtime() {
    let declaration = r#"<xsl:variable name="prefix" select="'global'"/><xsl:function name="f:make" as="element()"><xsl:param name="x"/><item><xsl:value-of select="concat($prefix, $x)"/></item></xsl:function>"#;
    assert_eq!(
        run(declaration, "f:make('!') instance of element(item)").unwrap(),
        "true"
    );
    assert_eq!(run(declaration, "string(f:make('!'))").unwrap(), "global!");
    assert_eq!(
        run(declaration, "empty(f:make('!')/parent::node())").unwrap(),
        "true"
    );
    assert_eq!(
        run(declaration, "f:make('!') is f:make('!')").unwrap(),
        "false"
    );
}

#[test]
fn functions_can_construct_typed_text_and_sorted_node_variables() {
    let declaration = r#"<xsl:function name="f:first" as="xs:string"><xsl:param name="nodes" as="element()*"/><xsl:variable name="sorted" as="element()*"><xsl:for-each select="$nodes"><xsl:sort select="." data-type="number" order="descending"/><xsl:sequence select="."/></xsl:for-each></xsl:variable><xsl:value-of select="$sorted[1]"/></xsl:function>"#;
    assert_eq!(run(declaration, "f:first(r/n)").unwrap(), "3");
}

#[test]
fn constructor_functions_keep_lexical_scope_through_indirect_calls() {
    let declaration = r#"<xsl:variable name="v" select="'global'"/><xsl:function name="f:make" as="element()"><item><xsl:value-of select="$v"/></item></xsl:function><xsl:function name="f:call" as="element()"><xsl:variable name="v" select="'local'"/><xsl:sequence select="(function(){f:make()})()"/></xsl:function>"#;
    assert_eq!(run(declaration, "string(f:call())").unwrap(), "global");
    assert_eq!(run(declaration, "string((f:make#0)())").unwrap(), "global");
}

#[test]
fn constructor_functions_obey_temporary_tree_and_recursion_limits() {
    use rust_xslt::{Limits, TransformOptions};
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" version="3.0"><xsl:function name="f:make"><item>hello</item></xsl:function><xsl:function name="f:recurse"><xsl:sequence select="f:recurse()"/><item/></xsl:function><xsl:template match="/"><xsl:sequence select="f:make()"/></xsl:template></xsl:stylesheet>"#;
    let options = TransformOptions {
        limits: Limits {
            max_temporary_bytes: 10,
            ..Limits::default()
        },
        ..TransformOptions::default()
    };
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform_with_options("<r/>", &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
    let source = source.replace("select=\"f:make()\"", "select=\"f:recurse()\"");
    let options = TransformOptions {
        limits: Limits {
            max_depth: 8,
            ..Limits::default()
        },
        ..TransformOptions::default()
    };
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform_with_options("<r/>", &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}

#[test]
fn constructor_functions_inside_nested_instruction_runtimes_are_bounded() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" version="3.0"><xsl:function name="f:make"><item/></xsl:function><xsl:template match="/"><xsl:try><xsl:try><xsl:try><xsl:sequence select="f:make()"/><xsl:catch><xsl:sequence select="error()"/></xsl:catch></xsl:try><xsl:catch><xsl:sequence select="error()"/></xsl:catch></xsl:try><xsl:catch><xsl:sequence select="error()"/></xsl:catch></xsl:try></xsl:template></xsl:stylesheet>"#;
    let error = Stylesheet::compile(source)
        .unwrap()
        .transform("<r/>")
        .unwrap_err();
    assert!(matches!(error.code.as_str(), "XPDY0130" | "FOER0000"));
}

#[cfg(all(unix, any(target_arch = "x86_64", target_arch = "aarch64")))]
#[test]
fn nested_function_constructors_work_on_a_bounded_thread_stack() {
    let result = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" exclude-result-prefixes="f" version="3.0">
              <xsl:output omit-xml-declaration="yes"/>
              <xsl:function name="f:a"><a><xsl:sequence select="f:b()"/></a></xsl:function>
              <xsl:function name="f:b"><b><xsl:sequence select="f:c()"/></b></xsl:function>
              <xsl:function name="f:c"><c><xsl:sequence select="f:d()"/></c></xsl:function>
              <xsl:function name="f:d"><d>ok</d></xsl:function>
              <xsl:template match="/"><xsl:try><xsl:sequence select="f:a()"/><xsl:catch><bad/></xsl:catch></xsl:try></xsl:template>
            </xsl:stylesheet>"#;
            Stylesheet::compile(source).unwrap().transform("<r/>").unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(result, "<a><b><c><d>ok</d></c></b></a>");
}

#[test]
fn string_typed_value_of_functions_support_recursive_calls_without_changing_node_results() {
    let declaration = r#"<xsl:function name="f:text" as="xs:string?"><xsl:param name="n" as="xs:integer"/><xsl:choose><xsl:when test="$n = 0"><xsl:value-of select="('a', 'b')"/></xsl:when><xsl:otherwise><xsl:value-of select="f:text($n - 1)"/></xsl:otherwise></xsl:choose></xsl:function>"#;
    assert_eq!(run(declaration, "f:text(12)").unwrap(), "a b");
    assert_eq!(run(declaration, "f:text(20)").unwrap_err().code, "XPDY0130");
    assert_eq!(
        run(declaration, "f:text(0) instance of xs:string").unwrap(),
        "true"
    );
    let node =
        r#"<xsl:function name="f:node" as="item()"><xsl:value-of select="'x'"/></xsl:function>"#;
    assert_eq!(run(node, "f:node() instance of text()").unwrap(), "true");
    let invalid = r#"<xsl:function name="f:two" as="xs:string"><xsl:value-of select="'a'"/><xsl:value-of select="'b'"/></xsl:function>"#;
    assert_eq!(run(invalid, "f:two()").unwrap_err().code, "XTTE0780");
}
