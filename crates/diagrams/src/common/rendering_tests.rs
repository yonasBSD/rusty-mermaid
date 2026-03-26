    use super::*;

    #[test]
    fn node_style_uses_theme() {
        let theme = Theme::light();
        let s = node_style(&theme);
        assert_eq!(s.fill, Some(theme.node_fill));
        assert_eq!(s.stroke, Some(theme.node_stroke));
        assert_eq!(s.stroke_width, Some(theme.default_stroke_width));
    }

    #[test]
    fn overlay_style_replaces_set_fields() {
        let mut base = Style {
            fill: Some(Color::WHITE),
            stroke: Some(Color::BLACK),
            stroke_width: Some(1.0),
            ..Default::default()
        };
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        overlay_style(&mut base, &custom);
        assert_eq!(base.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(base.stroke, Some(Color::BLACK)); // unchanged
    }

    #[test]
    fn apply_style_properties_parses_css() {
        let props = vec![
            StyleProperty { key: "fill".into(), value: "#f9f".into() },
            StyleProperty { key: "stroke-width".into(), value: "4px".into() },
            StyleProperty { key: "opacity".into(), value: "0.5".into() },
        ];
        let mut style = Style::default();
        apply_style_properties(&mut style, &props);
        assert!(style.fill.is_some());
        assert_eq!(style.stroke_width, Some(4.0));
        assert_eq!(style.opacity, Some(0.5));
    }

    #[test]
    fn merge_custom_style_with_none() {
        let theme = Theme::light();
        let s = merge_custom_style(None, &theme);
        assert_eq!(s.fill, Some(theme.node_fill));
    }

    #[test]
    fn merge_custom_style_overrides() {
        let theme = Theme::light();
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        let s = merge_custom_style(Some(&custom), &theme);
        assert_eq!(s.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(s.stroke, Some(theme.node_stroke));
    }

    #[test]
    fn contrasting_label_dark_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(20, 20, 20)), &theme);
        assert_eq!(lstyle.fill, Some(Color::WHITE));
    }

    #[test]
    fn contrasting_label_light_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(250, 250, 250)), &theme);
        assert_eq!(lstyle.fill, Some(Color::BLACK));
    }

    #[test]
    fn contrasting_label_mid_fill() {
        let theme = Theme::light();
        // rgb(180, 180, 180) has luminance ~0.46 — in the mid range, no override
        let lstyle = contrasting_label_style(Some(Color::rgb(180, 180, 180)), &theme);
        assert_eq!(lstyle.fill, Some(theme.node_text)); // unchanged
    }

    // -- Marker inset & path shortening tests --

    #[test]
    fn marker_inset_all_markers_have_positive_inset() {
        for m in [
            MarkerType::ArrowPoint,
            MarkerType::ArrowBarb,
            MarkerType::ArrowOpen,
            MarkerType::Cross,
            MarkerType::Circle,
            MarkerType::Aggregation,
            MarkerType::Composition,
            MarkerType::Dependency,
        ] {
            assert!(MARKER_INSET_VB > 0.0);
            assert!(marker_inset_px(m, 1.5) > 0.0, "{m:?} normal");
            assert!(marker_inset_px(m, 3.5) > 0.0, "{m:?} thick");
        }
    }

    #[test]
    fn marker_inset_px_scales_with_stroke_width() {
        let normal = marker_inset_px(MarkerType::ArrowPoint, 1.5);
        let thick = marker_inset_px(MarkerType::ArrowPoint, 3.5);
        assert!(thick > normal);
        let ratio = thick / normal;
        assert!((ratio - 3.5 / 1.5).abs() < 0.01);
    }

    #[test]
    fn shorten_path_end_pulls_back_line() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_end(&mut segs, 10.0);
        if let PathSegment::LineTo(p) = segs[1] {
            assert!((p.y - 90.0).abs() < 0.01);
        } else {
            panic!("expected LineTo");
        }
    }

    #[test]
    fn shorten_path_end_cascades_through_short_segment() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 70.0),
                to: Point::new(0.0, 96.0),
            },
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_end(&mut segs, 6.0);
        assert_eq!(segs.len(), 2);
        if let PathSegment::CubicTo { to, .. } = segs[1] {
            assert!((to.y - 94.0).abs() < 0.1);
        }
    }

    #[test]
    fn shorten_path_start_pulls_forward() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_start(&mut segs, 10.0);
        if let PathSegment::MoveTo(p) = segs[0] {
            assert!((p.y - 10.0).abs() < 0.01);
        }
    }

    #[test]
    fn shorten_path_start_cascades_through_short_segment() {
        // When the first LineTo is shorter than the requested shortening,
        // absorb it and continue into the next segment.
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 4.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 70.0),
                to: Point::new(0.0, 100.0),
            },
        ];
        shorten_path_start(&mut segs, 6.0);
        // First LineTo (len=4) absorbed, remaining 2.0 pulled into CubicTo
        assert_eq!(segs.len(), 2);
        if let PathSegment::MoveTo(p) = segs[0] {
            // Started at (0,4) after absorbing, pulled 2.0 toward cp1 (0,20)
            assert!((p.y - 6.0).abs() < 0.01);
        } else {
            panic!("expected MoveTo");
        }
    }

    #[test]
    fn shorten_path_end_cascades_through_micro_cubic() {
        // Basis spline interpolation can produce micro CubicTo segments near
        // endpoints. The cascade must absorb these so shortening doesn't silently
        // fail. Regression: resume edge in state_history_in_composite had a
        // 0.07px CubicTo that defeated the entire 3.15px shortening.
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 30.0),
                cp2: Point::new(0.0, 80.0),
                to: Point::new(0.0, 96.0),
            },
            // Micro cubic: only 0.07px
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 96.03),
                cp2: Point::new(0.0, 96.05),
                to: Point::new(0.0, 96.07),
            },
        ];
        shorten_path_end(&mut segs, 3.15);
        // Micro cubic (0.07px) absorbed, remaining ~3.08px pulled from main cubic
        assert_eq!(segs.len(), 2, "micro cubic should be absorbed");
        let end = prev_endpoint(&segs).unwrap();
        assert!(
            (end.y - (96.0 - 3.08)).abs() < 0.5,
            "endpoint should be pulled back into preceding cubic, got y={:.2}",
            end.y
        );
    }

    #[test]
    fn shorten_path_start_cascades_through_micro_cubic() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            // Micro cubic: only 0.05px
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 0.02),
                cp2: Point::new(0.0, 0.04),
                to: Point::new(0.0, 0.05),
            },
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 80.0),
                to: Point::new(0.0, 100.0),
            },
        ];
        shorten_path_start(&mut segs, 3.15);
        assert_eq!(segs.len(), 2, "micro cubic should be absorbed");
        if let PathSegment::MoveTo(p) = segs[0] {
            assert!(
                p.y > 3.0,
                "start should be pulled forward past micro cubic, got y={:.2}",
                p.y
            );
        } else {
            panic!("expected MoveTo");
        }
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_point() -> impl Strategy<Value = Point> {
            (-500.0..500.0_f64, -500.0..500.0_f64)
                .prop_map(|(x, y)| Point::new(x, y))
        }

        fn arb_straight_path(min_len: f64) -> impl Strategy<Value = Vec<PathSegment>> {
            (arb_point(), min_len..500.0_f64).prop_map(|(start, len)| {
                vec![
                    PathSegment::MoveTo(start),
                    PathSegment::LineTo(Point::new(start.x, start.y + len)),
                ]
            })
        }

        fn arb_cascade_path() -> impl Strategy<Value = (Vec<PathSegment>, f64, f64)> {
            (
                arb_point(),
                50.0..200.0_f64,
                1.0..10.0_f64,
                0.01..0.9_f64,
            )
                .prop_map(|(start, span, short_len, extra_frac)| {
                    let cubic_tangent = span * 0.1;
                    let cubic_end_y = start.y + span;
                    let final_y = cubic_end_y + short_len;
                    let dist = short_len + extra_frac * cubic_tangent;
                    let segs = vec![
                        PathSegment::MoveTo(start),
                        PathSegment::CubicTo {
                            cp1: Point::new(start.x, start.y + span * 0.3),
                            cp2: Point::new(start.x, cubic_end_y - cubic_tangent),
                            to: Point::new(start.x, cubic_end_y),
                        },
                        PathSegment::LineTo(Point::new(start.x, final_y)),
                    ];
                    (segs, dist, final_y)
                })
        }

        proptest! {
            #[test]
            fn shorten_end_distance_exact(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let actual = orig_end.distance_to(new_end);
                prop_assert!((actual - dist).abs() < 0.01);
            }

            #[test]
            fn shorten_end_stays_collinear(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let start = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let cross = ((orig_end.x - start.x) * (new_end.y - start.y)
                           - (orig_end.y - start.y) * (new_end.x - start.x)).abs();
                prop_assert!(cross < 0.01);
            }

            #[test]
            fn shorten_end_never_overshoots(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let start = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                prop_assert!(start.distance_to(new_end) < start.distance_to(orig_end));
                prop_assert!(start.distance_to(new_end) > 0.0);
            }

            #[test]
            fn shorten_start_distance_exact(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let orig = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                shorten_path_start(&mut segs, dist);
                let new = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                prop_assert!((orig.distance_to(new) - dist).abs() < 0.01);
            }

            #[test]
            fn cascade_retraction_correct(
                (mut segs, dist, orig_final_y) in arb_cascade_path(),
            ) {
                let orig_end = Point::new(
                    match segs[0] { PathSegment::MoveTo(p) => p.x, _ => unreachable!() },
                    orig_final_y,
                );
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let actual = orig_end.distance_to(new_end);
                prop_assert!((actual - dist).abs() < 1.5);
            }

            #[test]
            fn inset_scales_linearly(
                sw1 in 0.5..5.0_f64,
                sw2 in 0.5..5.0_f64,
            ) {
                let r1 = marker_inset_px(MarkerType::ArrowPoint, sw1) / sw1;
                let r2 = marker_inset_px(MarkerType::ArrowPoint, sw2) / sw2;
                prop_assert!((r1 - r2).abs() < 0.001);
            }

            #[test]
            fn bidir_shortening_independent(
                dist in 1.0..20.0_f64,
            ) {
                let mut segs = vec![
                    PathSegment::MoveTo(Point::new(0.0, 0.0)),
                    PathSegment::LineTo(Point::new(0.0, 100.0)),
                ];
                shorten_path_start(&mut segs, dist);
                shorten_path_end(&mut segs, dist);
                let s = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let e = prev_endpoint(&segs).unwrap();
                prop_assert!((s.distance_to(e) - (100.0 - 2.0 * dist)).abs() < 0.01);
            }
        }
    }
