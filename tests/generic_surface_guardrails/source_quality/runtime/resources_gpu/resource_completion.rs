use super::{prelude_source, radiant_source};

#[test]
fn resource_completions_use_named_parts_for_request_results() {
    let source = radiant_source("src/runtime/resource/load.rs");
    let resource = radiant_source("src/runtime/resource.rs");
    let runtime = radiant_source("src/runtime/mod.rs");
    let prelude = prelude_source();
    let update_context_business_resource =
        radiant_source("src/application/runtime/update_context/business/resource.rs");

    assert!(
        source.contains("pub struct ResourceCompletionParts")
            && source.contains("pub fn from_parts(parts: ResourceCompletionParts<T>) -> Self")
            && source.contains("Self::from_parts(ResourceCompletionParts { request, load })"),
        "resource completions should expose named parts and keep the compatibility constructor"
    );
    assert!(
        source.contains("ResourceCompletion::from_parts(ResourceCompletionParts {")
            && update_context_business_resource
                .contains("ResourceCompletion::from_parts(ResourceCompletionParts")
            && update_context_business_resource.contains("request: resource")
            && update_context_business_resource.contains("load,"),
        "resource completion mapping and business resource helpers should use the named-parts construction path"
    );
    assert!(
        resource.contains("ResourceCompletionParts")
            && runtime.contains("ResourceCompletionParts")
            && !prelude.contains("ResourceCompletionParts"),
        "resource completion parts should remain available from runtime ownership without entering the common prelude"
    );
}
