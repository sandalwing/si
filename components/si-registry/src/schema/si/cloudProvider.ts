import {
  RegistryEntry,
  SchematicKind,
  NodeKind,
  Arity,
  ValidatorKind,
} from "../../registryEntry";
import { onlyImplementation } from "../include/standardConceptInputs";

const cloudProvider: RegistryEntry = {
  entityType: "cloudProvider",
  nodeKind: NodeKind.Concept,
  ui: {
    menu: [
      {
        name: "cloud",
        menuCategory: ["provider"],
        schematicKind: SchematicKind.Deployment,
        rootEntityTypes: ["application"],
      },
    ],
  },
  inputs: [
    ...onlyImplementation,
    {
      name: "kubernetesCluster",
      types: ["kubernetesCluster"],
      edgeKind: "deployment",
      arity: Arity.Many,
    },
  ],
  properties: [
    {
      type: "string",
      name: "implementation",
      widget: {
        name: "selectFromInput",
        inputName: "implementations",
      },
      validation: [
        {
          kind: ValidatorKind.Required,
        },
      ],
    },
  ],
  actions: [{ name: "deploy" }],
};

export default cloudProvider;
