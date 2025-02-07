import _ from "npm:lodash";
import _logger from "../logger.ts";
import { createInputSocketFromProp } from "../spec/sockets.ts";
import { ExpandedPkgSpec } from "../spec/pkgs.ts";

const logger = _logger.ns("assetOverrides").seal();

export function assetSpecificOverrides(
  incomingSpecs: ExpandedPkgSpec[],
): ExpandedPkgSpec[] {
  const newSpecs = [] as ExpandedPkgSpec[];

  for (const spec of incomingSpecs) {
    if (overrides.has(spec.name)) {
      logger.debug(`Running override for ${spec.name}`);
      overrides.get(spec.name)?.(spec);
    }
    newSpecs.push(spec);
  }

  return newSpecs;
}

type OverrideFn = (spec: ExpandedPkgSpec) => void;

const overrides = new Map<string, OverrideFn>([
  ["AWS::EC2::Route", (spec: ExpandedPkgSpec) => {
    addGatewayIdSocketToEC2Route(spec);
  }],
]);

function addGatewayIdSocketToEC2Route(spec: ExpandedPkgSpec) {
  const schema = spec.schemas[0];
  const variant = spec.schemas[0].variants[0];
  const domain = variant.domain;

  if (!schema || !variant || !domain || domain.kind !== "object") {
    throw new Error(`Unable to run override for ${spec.name}`);
  }
  for (const prop of domain.entries) {
    if (prop.name === "GatewayId") {
      const socket = createInputSocketFromProp(prop, "one");

      const data = socket.data;
      if (data) {
        const annotation = JSON.parse(data.connectionAnnotations);
        annotation.push({ tokens: ["InternetGatewayId"] });
        annotation.push({ tokens: ["VPNGatewayId"] });
        data.connectionAnnotations = JSON.stringify(annotation);
      }

      variant.sockets.push(socket);
    }
  }
}
