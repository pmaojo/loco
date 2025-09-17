use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use loco_rs::{
    boot,
    environment::Environment,
    ontology::{
        value_objects::Iri, Class, Individual, Ontology, Property, PropertyAssertion, PropertyKind,
    },
    tests_cfg::{config::test_config, db::AppHook},
};

#[tokio::test]
async fn context_exposes_reasoner_from_config() {
    let mut config = test_config();
    let seed_identifier = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let seed_path = std::env::temp_dir().join(format!("loco-ontology-{seed_identifier}.seed"));
    fs::write(&seed_path, b"{}\n").expect("seed file");
    config.ontology.seeds = vec![seed_path.clone()];

    let context = boot::create_context::<AppHook>(&Environment::Test, config)
        .await
        .expect("context creation");

    let repository = context.ontology.repository();
    let reasoner = context.ontology.reasoner();

    let ontology_id = Iri::new("https://example.org/ontology").expect("ontology iri");
    let base_class_id = Iri::new("https://example.org/Base").expect("base class");
    let derived_class_id = Iri::new("https://example.org/Derived").expect("derived class");
    let property_id = Iri::new("https://example.org/related").expect("related property");
    let alice_id = Iri::new("https://example.org/Alice").expect("alice");
    let bob_id = Iri::new("https://example.org/Bob").expect("bob");

    let mut ontology = Ontology::new(ontology_id.clone());
    let base_class = Class::new(base_class_id.clone());
    let mut derived_class = Class::new(derived_class_id.clone());
    derived_class.add_parent(base_class_id.clone());
    ontology.add_class(base_class).expect("base class");
    ontology.add_class(derived_class).expect("derived class");

    let mut property = Property::new(property_id.clone(), PropertyKind::Object);
    property.add_domain(base_class_id.clone());
    property.add_range(base_class_id.clone());
    ontology.add_property(property).expect("property");

    let mut alice = Individual::new(alice_id.clone());
    alice.assert_type(base_class_id.clone());
    alice.add_property_assertion(
        property_id.clone(),
        PropertyAssertion::Individual(bob_id.clone()),
    );
    ontology.add_individual(alice).expect("alice individual");

    let mut bob = Individual::new(bob_id.clone());
    bob.assert_type(base_class_id.clone());
    ontology.add_individual(bob).expect("bob individual");

    repository
        .insert(ontology)
        .await
        .expect("ontology inserted");

    let ancestors = reasoner
        .ancestors_of(&ontology_id, &derived_class_id)
        .await
        .expect("ancestors");
    assert_eq!(ancestors, vec![base_class_id.clone()]);

    let descendants = reasoner
        .descendants_of(&ontology_id, &base_class_id)
        .await
        .expect("descendants");
    assert_eq!(descendants, vec![derived_class_id.clone()]);

    let related = reasoner
        .related_individuals(&ontology_id, &property_id, &alice_id)
        .await
        .expect("related individuals");
    assert_eq!(related, vec![bob_id.clone()]);

    let shortest = reasoner
        .shortest_path(&ontology_id, &alice_id, &bob_id)
        .await
        .expect("shortest path");
    assert_eq!(shortest, Some(vec![alice_id.clone(), bob_id.clone()]));

    let _ = fs::remove_file(seed_path);
}
