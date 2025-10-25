// Integration tests for memory operations

mod memory {
    mod core {
        mod test_schema;
    }
    mod migration {
        mod test_converter;
    }
    mod monitoring {
        mod test_metrics;
        mod test_metrics_test;
        mod test_metrics_tests;
    }
    mod schema {
        mod test_relationship_schema;
    }
    mod transaction {
        mod test_transaction_manager_tests;
    }
    mod vector {
        mod test_vector_index;
        mod test_vector_repository;
    }
}
