pub mod name_generator {
    use std::{collections::HashSet, sync::Arc};
    use std::sync::Mutex;
    
    static ADJECTIVES: [&str; 6] = [
        "Mushy",
        "Starry",
        "Peaceful",
        "Phony",
        "Amazing",
        "Queasy",
    ];

    static ANIMALS: [&str; 6] = [
        "Owl",
        "Mantis",
        "Gopher",
        "Robin",
        "Vulture",
        "Prawn",    
    ];
    
    #[derive(Clone)]
    pub struct Names(Arc<Mutex<HashSet<String>>>);
    
    impl Names {
        pub fn new() -> Self {
            Self(Arc::new(Mutex::new(HashSet::new())))
        }
    
        pub fn insert(&self, name: String) -> bool {
            self.0.lock().unwrap().insert(name)
        }
    
        pub fn remove(&self, name: &String) {
            self.0.lock().unwrap().remove(name);
        }
    
        pub fn get_unique(&self) -> String {
            let mut name = Self::random_name();
            while !self.insert(name.clone()) {
                name = Self::random_name();
            }
    
            name
        }    

        fn random_name() -> String {
            let adjective = fastrand::choice(ADJECTIVES).unwrap();
            let animal = fastrand::choice(ANIMALS).unwrap();
        
            format!("{adjective}{animal}")
        }
    }
}
