use iac_forge::{IacResource, to_kebab_case};

/// Generate a helm-unittest test file for a resource chart.
#[must_use]
pub fn generate_deployment_test(resource: &IacResource) -> String {
    let chart_name = to_kebab_case(&resource.name);

    format!(
        r#"suite: {chart_name} deployment tests
templates:
  - templates/deployment.yaml
values:
  - ../values.yaml
tests:
  - it: should render a Deployment
    asserts:
      - isKind:
          of: Deployment

  - it: should set correct labels
    asserts:
      - isSubset:
          path: metadata.labels
          content:
            app.kubernetes.io/managed-by: Helm

  - it: should enforce security context
    asserts:
      - equal:
          path: spec.template.spec.securityContext.runAsNonRoot
          value: true

  - it: should set resource requests
    asserts:
      - isNotEmpty:
          path: spec.template.spec.containers[0].resources.requests
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::test_resource;

    #[test]
    fn generates_test_yaml() {
        let resource = test_resource("static_secret");
        let test = generate_deployment_test(&resource);
        assert!(test.contains("suite: static-secret"));
        assert!(test.contains("deployment.yaml"));
        assert!(test.contains("isKind"));
    }
}
