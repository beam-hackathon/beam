#[cfg(test)]
mod construct_tests {
    extern crate apache_beam;

    use apache_beam::construct::build_pipeline;
    use apache_beam::construct::PCollection;
    use apache_beam::construct::PTransform;
    use apache_beam::construct::Root;
    use apache_beam::transforms::Gbk;
    use apache_beam::transforms::Impulse;

    use apache_beam::runners::direct_runner::DirectRunner;
    use apache_beam::runners::direct_runner::Runner;

    #[test]
    fn apply_impulse() {
        let proto = build_pipeline(&|root: Root| {
            root.apply(&"impulse".to_string(), &Impulse {});
        });
        println!("{:#?}", proto);
    }

    #[test]
    fn apply_composite() {
        let proto = build_pipeline(&|root: Root| {
            // TODO: That's a lot of boilerplate with the lifetime annotations,
            // and they don't seem to be buying us much. Can we get rid of them?
            // (See other comment about storing an Rc<PipelineHandler> rather than
            // a bare ref...)
            struct RootComposite {}
            impl<'a> PTransform<'a, Root<'a>, PCollection<'a, String>> for RootComposite {
                fn expand(&self, root: &Root<'a>) -> PCollection<'a, String> {
                    root.apply(&"Inner".to_string(), &Impulse {});
                    root.apply(&"Returned".to_string(), &Impulse {})
                }
            }

            struct EmptyComposite {}
            impl<'a> PTransform<'a, PCollection<'a, String>, PCollection<'a, String>> for EmptyComposite {
                fn expand(&self, pcoll: &PCollection<'a, String>) -> PCollection<'a, String> {
                    pcoll.clone()
                }
            }

            root.apply(&"Root".to_string(), &RootComposite {})
                .apply(&"Empty".to_string(), &EmptyComposite {});
        });

        //println!("{:#?}", proto);
        assert_eq!(proto.root_transform_ids.len(), 2);
        let transforms = &proto.components.as_ref().unwrap().transforms;
        assert_eq!(transforms.len(), 4);

        let transform0 = transforms.get("transform0").as_ref().unwrap().clone();
        let transform2 = transforms.get("transform2").as_ref().unwrap().clone();
        let transform3 = transforms.get("transform3").as_ref().unwrap().clone();

        assert_eq!(transform0.outputs.get("out").unwrap(), "pcoll1");
        assert_eq!(transform0.unique_name, "Root");
        assert_eq!(transform0.subtransforms.len(), 2);

        assert_eq!(transform2.outputs.get("out").unwrap(), "pcoll1");
        assert_eq!(transform2.unique_name, "Root/Returned");
        assert_eq!(transform2.subtransforms.len(), 0);

        assert_eq!(transform3.inputs.get("pcoll1").unwrap(), "pcoll1");
        assert_eq!(transform3.unique_name, "Empty");
    }

    fn do_fn_str(s: &String) -> Vec<i32> {
        vec![s.len() as i32, 23, 24]
    }

    fn do_fn_int(n: &i32) -> Vec<String> {
        vec!["1".to_string(), n.to_string()]
    }

    // cargo test construct_tests::flat_map -- --nocapture
    #[test]
    fn flat_map() {
        println!("\nhello!\n");
        let proto = build_pipeline(&|root: Root| {
            root.apply(&"impulse".to_string(), &Impulse {})
                .flat_map(&"map 1".to_string(), do_fn_str)
                .flat_map(&"map 2".to_string(), do_fn_int);
        });

        println!("{:#?}", proto);

        println!("\nhello!\n");
    }

    #[test]
    fn direct_runner() -> Result<(), String> {
        let dr = DirectRunner {};
        dr.run(&|root: Root| {
            root.apply(&"impulse".to_string(), &Impulse {});
        })?;
        Ok(())
    }

    #[test]
    fn test_word_count() -> Result<(), String> {
        let dr = DirectRunner {};
        dr.run(&|root: Root| {
            root.apply(&"impulse".to_string(), &Impulse {})
            .flat_map(&"Create".to_string(), |_unused : &String| {vec![
                "In the beginning God created the heaven and the earth.".to_string(),
                "And the earth was without form, and void; and darkness was upon the face of the deep.".to_string(),
                "And the Spirit of God moved upon the face of the waters.".to_string(),
                "And God said, Let there be light: and there was light.".to_string(),
            ]})
            .map(&"Normalize".to_string(), |line| {line.to_lowercase()})
            .flat_map(&"Split".to_string(), |line| {line.split(" ").map(|s| s.to_string()).collect::<Vec<String>>()})
            .map(&"PairWithOne".to_string(), |word| {(word.to_string(), 1)})
            .apply(&"GroupByKey".to_string(), &Gbk{tuple_type: "StringInt32".to_string()})
            .map(&"Sum".to_string(),
                |grouped_per_word| {(grouped_per_word.0.to_string(), grouped_per_word.1.iter().sum::<i32>())})
            .map(&"Format".to_string(),
                |count_per_word| {format!("{}: {}", count_per_word.0, count_per_word.1)})
            .map(&"Print".to_string(), |result| {println!("RESULT {}", result)})
            ;
        })?;
        Ok(())
    }
}
