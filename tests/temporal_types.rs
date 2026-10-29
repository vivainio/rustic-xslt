use rust_xslt::Stylesheet;
fn run(expr: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#)).unwrap().transform("<r d='2001-02-03'/>")
}
#[test]
fn constructors_use_canonical_lexical_forms() {
    for (expr, expected) in [
        ("xs:dateTime('2001-02-03T04:05:06')", "2001-02-03T04:05:06"),
        (
            "xs:dateTime(' 2001-02-03T04:05:06.500+05:30 ')",
            "2001-02-03T04:05:06.5+05:30",
        ),
        (
            "xs:dateTime('2001-02-03T24:00:00Z')",
            "2001-02-04T00:00:00Z",
        ),
        (
            "xs:dateTime('2001-02-03T04:05:06+00:00')",
            "2001-02-03T04:05:06Z",
        ),
        ("xs:date('2000-02-29')", "2000-02-29"),
        ("xs:date('-0044-03-15Z')", "-0044-03-15Z"),
        ("xs:time('23:59:59.120')", "23:59:59.12"),
        ("xs:time('24:00:00')", "00:00:00"),
        (
            "xs:date(xs:dateTime('2001-02-03T23:00:00-05:00'))",
            "2001-02-03-05:00",
        ),
        (
            "xs:time(xs:dateTime('2001-02-03T23:00:00-05:00'))",
            "23:00:00-05:00",
        ),
        ("xs:dateTime(xs:date('2001-02-03'))", "2001-02-03T00:00:00"),
        ("xs:duration('P1Y2M3DT4H5M6.7S')", "P1Y2M3DT4H5M6.7S"),
        ("xs:duration('P14M')", "P1Y2M"),
        ("xs:duration('PT36H')", "P1DT12H"),
        ("xs:duration('-P0D')", "PT0S"),
        ("xs:yearMonthDuration('P0Y')", "P0M"),
        ("xs:yearMonthDuration('-P13M')", "-P1Y1M"),
        ("xs:dayTimeDuration('PT90M')", "PT1H30M"),
        ("xs:yearMonthDuration(xs:duration('P1Y2M3D'))", "P1Y2M"),
        ("xs:dayTimeDuration(xs:duration('P1Y2M3D'))", "P3D"),
        ("xs:date(r/@d)", "2001-02-03"),
        ("empty(xs:date(()))", "true"),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
}
#[test]
fn invalid_lexical_forms_and_casts_are_rejected() {
    for expr in [
        "xs:date('2001-02-30')",
        "xs:date('2001-13-01')",
        "xs:date('2001-2-03')",
        "xs:date('02001-01-01')",
        "xs:time('25:00:00')",
        "xs:time('24:00:01')",
        "xs:time('12:60:00')",
        "xs:dateTime('2001-02-03')",
        "xs:dateTime('2001-02-03T04:05:06+15:00')",
        "xs:duration('P')",
        "xs:duration('PT')",
        "xs:duration('P1YT')",
        "xs:duration('1Y')",
        "xs:yearMonthDuration('P1D')",
        "xs:dayTimeDuration('P1Y')",
        "xs:date(1)",
        "xs:time(xs:date('2001-02-03'))",
        "xs:date(xs:time('01:02:03'))",
        "xs:dateTime(xs:duration('P1D'))",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn comparison_normalizes_time_zones_and_orders_durations() {
    for (expr, expected) in [
        (
            "xs:dateTime('2001-02-03T10:00:00+01:00') = xs:dateTime('2001-02-03T09:00:00Z')",
            "true",
        ),
        ("xs:date('2001-02-03') lt xs:date('2001-02-04')", "true"),
        ("xs:time('10:00:00') &lt; xs:time('09:00:00Z')", "false"),
        ("xs:duration('PT60M') = xs:duration('PT1H')", "true"),
        ("xs:duration('P1Y') = xs:duration('P12M')", "true"),
        (
            "xs:yearMonthDuration('P1Y') &lt; xs:yearMonthDuration('P13M')",
            "true",
        ),
        (
            "xs:dayTimeDuration('PT1H') gt xs:dayTimeDuration('PT59M')",
            "true",
        ),
        (
            "xs:yearMonthDuration('P12M') = xs:dayTimeDuration('PT0S')",
            "false",
        ),
        ("xs:date(r/@d) = r/@d", "true"),
        (
            "deep-equal(xs:date('2001-02-03'), xs:date('2001-02-03'))",
            "true",
        ),
        (
            "distinct-values((xs:date('2001-02-03'), xs:date('2001-02-03Z'), xs:date('2001-02-03')))",
            "2001-02-03",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "xs:date('2001-02-03') = xs:time('01:00:00')",
        "xs:duration('P1D') lt xs:duration('P2D')",
        "xs:date('2001-02-03') = 1",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn types_instances_and_accessors() {
    for (expr, expected) in [
        ("xs:date('2001-02-03') instance of xs:date", "true"),
        ("xs:date('2001-02-03') instance of xs:dateTime", "false"),
        (
            "xs:yearMonthDuration('P1Y') instance of xs:duration",
            "true",
        ),
        (
            "xs:duration('P1Y') instance of xs:yearMonthDuration",
            "false",
        ),
        ("xs:time('01:02:03') instance of xs:anyAtomicType", "true"),
        ("xs:date('2001-02-03') castable as xs:dateTime", "true"),
        ("'2001-02-30' castable as xs:date", "false"),
        (
            "string(xs:date('2001-02-03') cast as xs:dateTime)",
            "2001-02-03T00:00:00",
        ),
        (
            "year-from-dateTime(xs:dateTime('2001-02-03T04:05:06.25-07:00'))",
            "2001",
        ),
        ("month-from-date(xs:date('2001-02-03'))", "2"),
        ("day-from-dateTime(xs:dateTime('2001-02-03T04:05:06'))", "3"),
        ("hours-from-time(xs:time('04:05:06'))", "4"),
        (
            "minutes-from-dateTime(xs:dateTime('2001-02-03T04:05:06'))",
            "5",
        ),
        (
            "seconds-from-dateTime(xs:dateTime('2001-02-03T04:05:06.25'))",
            "6.25",
        ),
        (
            "timezone-from-date(xs:date('2001-02-03-05:30'))",
            "-PT5H30M",
        ),
        ("empty(timezone-from-time(xs:time('01:02:03')))", "true"),
        ("empty(year-from-date(()))", "true"),
        ("years-from-duration(xs:duration('-P14M'))", "-1"),
        ("months-from-duration(xs:duration('-P14M'))", "-2"),
        ("days-from-duration(xs:duration('P1DT36H'))", "2"),
        ("hours-from-duration(xs:duration('P1DT36H'))", "12"),
        ("minutes-from-duration(xs:duration('PT90M'))", "30"),
        ("seconds-from-duration(xs:duration('-PT1M1.5S'))", "-1.5"),
        ("implicit-timezone()", "PT0S"),
        ("current-dateTime() = current-dateTime()", "true"),
        ("current-date() instance of xs:date", "true"),
        ("current-time() instance of xs:time", "true"),
        ("xs:date(current-dateTime()) = current-date()", "true"),
        (
            "timezone-from-dateTime(current-dateTime()) instance of xs:dayTimeDuration",
            "true",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "year-from-date(xs:dateTime('2001-02-03T00:00:00'))",
        "year-from-date(1)",
        "seconds-from-duration(xs:date('2001-02-03'))",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn oversized_years_and_durations_fail_without_panicking() {
    for expr in [
        "xs:date('99999999999999999999-01-01')",
        "xs:date('2000000000-01-01')",
        "xs:duration('P99999999999999999999999999999D')",
        "xs:duration('P9999999999999999999Y')",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn arithmetic_adjusts_calendar_values_and_durations() {
    for (expr, expected) in [
        (
            "xs:dateTime('2000-01-31T10:00:00') + xs:yearMonthDuration('P1M')",
            "2000-02-29T10:00:00",
        ),
        (
            "xs:date('2000-03-31') - xs:yearMonthDuration('P1M')",
            "2000-02-29",
        ),
        (
            "xs:dateTime('2000-12-31T23:59:59Z') + xs:dayTimeDuration('PT1S')",
            "2001-01-01T00:00:00Z",
        ),
        (
            "xs:dayTimeDuration('P1D') + xs:date('2000-02-28')",
            "2000-02-29",
        ),
        (
            "xs:date('2000-02-28') + xs:dayTimeDuration('PT36H')",
            "2000-02-29",
        ),
        (
            "xs:time('23:30:00') + xs:dayTimeDuration('PT1H')",
            "00:30:00",
        ),
        (
            "xs:time('00:30:00') - xs:dayTimeDuration('PT1H')",
            "23:30:00",
        ),
        (
            "xs:dateTime('2000-01-01T00:00:00Z') - xs:dateTime('1999-12-31T00:00:00Z')",
            "P1D",
        ),
        ("xs:date('2000-03-01') - xs:date('2000-02-01')", "P29D"),
        ("xs:time('12:00:00') - xs:time('10:30:00')", "PT1H30M"),
        ("xs:time('10:00:00+02:00') - xs:time('10:00:00Z')", "-PT2H"),
        (
            "xs:yearMonthDuration('P1Y') + xs:yearMonthDuration('P2M')",
            "P1Y2M",
        ),
        (
            "xs:dayTimeDuration('PT1H') - xs:dayTimeDuration('PT2H')",
            "-PT1H",
        ),
        ("xs:yearMonthDuration('P1Y') * 2.5", "P2Y6M"),
        ("3 * xs:dayTimeDuration('PT20M')", "PT1H"),
        ("xs:dayTimeDuration('PT1H') div 4", "PT15M"),
        (
            "xs:yearMonthDuration('P2Y') div xs:yearMonthDuration('P6M')",
            "4",
        ),
        (
            "xs:dayTimeDuration('PT1H') div xs:dayTimeDuration('PT15M')",
            "4",
        ),
        ("empty(xs:date('2000-01-01') + ())", "true"),
        (
            "dateTime(xs:date('2000-01-02'), xs:time('03:04:05'))",
            "2000-01-02T03:04:05",
        ),
        (
            "dateTime(xs:date('2000-01-02Z'), xs:time('03:04:05'))",
            "2000-01-02T03:04:05Z",
        ),
        (
            "dateTime(xs:date('2000-01-02+01:00'), xs:time('03:04:05+01:00'))",
            "2000-01-02T03:04:05+01:00",
        ),
        ("empty(dateTime((), xs:time('03:04:05')))", "true"),
        (
            "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00'))",
            "2000-01-01T10:00:00Z",
        ),
        (
            "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00Z'), xs:dayTimeDuration('-PT5H'))",
            "2000-01-01T05:00:00-05:00",
        ),
        (
            "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00+05:00'), ())",
            "2000-01-01T10:00:00",
        ),
        (
            "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00'), xs:dayTimeDuration('PT3H'))",
            "2000-01-01T10:00:00+03:00",
        ),
        (
            "adjust-date-to-timezone(xs:date('2000-01-01Z'), xs:dayTimeDuration('-PT5H'))",
            "1999-12-31-05:00",
        ),
        (
            "adjust-time-to-timezone(xs:time('01:00:00Z'), xs:dayTimeDuration('-PT5H'))",
            "20:00:00-05:00",
        ),
        ("empty(adjust-time-to-timezone(()))", "true"),
        (
            "min((xs:date('2001-02-03'), xs:date('2000-01-01')))",
            "2000-01-01",
        ),
        (
            "max((xs:dayTimeDuration('PT1H'), xs:dayTimeDuration('PT61M')))",
            "PT1H1M",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "xs:date('2000-01-01') + xs:date('2000-01-02')",
        "xs:time('10:00:00') + xs:yearMonthDuration('P1M')",
        "xs:date('2000-01-01') - xs:time('10:00:00')",
        "xs:yearMonthDuration('P1Y') * xs:yearMonthDuration('P1Y')",
        "xs:yearMonthDuration('P1Y') div 0",
        "xs:dayTimeDuration('P1D') * xs:double('NaN')",
        "xs:dayTimeDuration('P1D') div xs:dayTimeDuration('PT0S')",
        "xs:yearMonthDuration('P1Y') div xs:dayTimeDuration('P1D')",
        "dateTime(xs:date('2000-01-01Z'), xs:time('01:00:00+01:00'))",
        "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00'), xs:dayTimeDuration('PT15H'))",
        "adjust-dateTime-to-timezone(xs:dateTime('2000-01-01T10:00:00'), xs:dayTimeDuration('PT1M1S'))",
        "adjust-date-to-timezone(xs:dateTime('2000-01-01T10:00:00'))",
        "xs:date('999999999-12-31') + xs:dayTimeDuration('P999999999D')",
        "min((xs:date('2000-01-01'), xs:time('10:00:00')))",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn gregorian_fragments_have_canonical_forms_equality_and_casts() {
    for (expr, expected) in [
        ("xs:gYear('2001')", "2001"),
        ("xs:gYear(' -0044Z ')", "-0044Z"),
        ("xs:gYearMonth('2001-02+05:00')", "2001-02+05:00"),
        ("xs:gMonth('--02')", "--02"),
        ("xs:gMonthDay('--02-29')", "--02-29"),
        ("xs:gDay('---31Z')", "---31Z"),
        ("xs:gYear(xs:dateTime('2001-02-03T04:05:06Z'))", "2001Z"),
        ("xs:gYearMonth(xs:date('2001-02-03'))", "2001-02"),
        ("xs:gMonth(xs:date('2001-02-03-05:00'))", "--02-05:00"),
        (
            "xs:gMonthDay(xs:dateTime('2001-02-03T00:00:00'))",
            "--02-03",
        ),
        ("xs:gDay(xs:date('2001-02-03'))", "---03"),
        ("xs:gYear('2001') eq xs:gYear('2001')", "true"),
        ("xs:gYear('2001Z') eq xs:gYear('2001+00:00')", "true"),
        ("xs:gYear('2001+01:00') eq xs:gYear('2001Z')", "false"),
        ("xs:gMonthDay('--02-03') ne xs:gMonthDay('--02-04')", "true"),
        ("xs:gDay('---05') = xs:gDay('---05')", "true"),
        ("xs:gMonth('--05') instance of xs:gMonth", "true"),
        ("xs:gMonth('--05') instance of xs:gDay", "false"),
        ("xs:gDay('---05') instance of xs:anyAtomicType", "true"),
        ("'--13' castable as xs:gMonth", "false"),
        (
            "string(xs:gYearMonth('2001-02') cast as xs:gYearMonth)",
            "2001-02",
        ),
        (
            "count(distinct-values((xs:gYear('2001'), xs:gYear('2001'))))",
            "1",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "xs:gYear('01')",
        "xs:gYear('02001')",
        "xs:gYearMonth('2001-13')",
        "xs:gMonth('2')",
        "xs:gMonthDay('--02-30')",
        "xs:gMonthDay('--04-31')",
        "xs:gDay('---32')",
        "xs:gDay('---00')",
        "xs:gDay('--05')",
        "xs:gYear(xs:time('01:02:03'))",
        "xs:gYear(xs:gMonth('--01'))",
        "xs:dateTime(xs:gYear('2001'))",
        "xs:gYear('2001') lt xs:gYear('2002')",
        "xs:gDay('---01') >= xs:gDay('---01')",
        "xs:gYear('2001') = xs:gDay('---01')",
        "min((xs:gYear('2001'), xs:gYear('2002')))",
    ] {
        assert!(run(expr.replace(">=", "&gt;=").as_str()).is_err(), "{expr}");
    }
}
