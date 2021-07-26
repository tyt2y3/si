import { OpSource, OpType, ResourceInternalHealth, SiEntity } from "si-entity";
import {
  SyncResourceRequest,
  CommandProtocolFinish,
} from "../controllers/syncResource";
import { SiCtx } from "../siCtx";
import WebSocket from "ws";
import _ from "lodash";
import {
  awsAccessKeysEnvironment,
  awsRegion,
  findEntityByType,
} from "../support";
import {
  DiscoveryProtocolFinish,
  DiscoveryRequest,
} from "../controllers/discover";
import Debug from "debug";

const debug = Debug("veritech:intel:awsEks");

export async function syncResource(
  _ctx: typeof SiCtx,
  req: SyncResourceRequest,
  _ws: WebSocket,
): Promise<CommandProtocolFinish["finish"]> {
  const response: CommandProtocolFinish["finish"] = {
    data: {},
    state: req.resource.state,
    health: req.resource.health,
    internalStatus: req.resource.internalStatus,
    internalHealth: req.resource.internalHealth,
    subResources: req.resource.subResources,
  };

  const cluster = findEntityByType(req, "awsEksCluster");

  if (!cluster) {
    response.error = "No cluster connected";
    response.state = "error";
    response.health = "error";
    response.internalHealth = ResourceInternalHealth.Error;
    return response;
  }
  const clusterResource = _.find(
    req.resourceContext,
    (r) => r.entityId == cluster.id,
  );
  if (clusterResource) {
    response.data = clusterResource.data;
    response.error = clusterResource.error;
    response.state = clusterResource.state;
    response.health = clusterResource.health;
    response.internalHealth = clusterResource.internalHealth;
    response.internalStatus = clusterResource.internalStatus;
    response.subResources = clusterResource.subResources;
  } else {
    response.state = "unknown";
    response.health = "unknown";
    response.internalHealth = ResourceInternalHealth.Unknown;
    return response;
  }
  return response;
}

export async function discover(
  ctx: typeof SiCtx,
  req: DiscoveryRequest,
  _ws: WebSocket,
): Promise<DiscoveryProtocolFinish["finish"]> {
  const response: DiscoveryProtocolFinish["finish"] = {
    discovered: [],
  };
  const awsEnv = awsAccessKeysEnvironment(req);
  const region = awsRegion(req);
  const output = await ctx.exec(
    "aws",
    ["eks", "--region", region, "list-clusters"],
    {
      env: awsEnv,
    },
  );

  let listClusters: Record<string, string[]>;
  try {
    listClusters = JSON.parse(output.stdout);
  } catch (e) {
    debug("list-clusters output %O", output);
    debug(e);
    throw e;
  }
  if (listClusters["clusters"]) {
    for (const cluster of listClusters["clusters"]) {
      const clusterOutput = await ctx.exec(
        "aws",
        ["eks", "--region", region, "describe-cluster", "--name", cluster],
        { env: awsEnv },
      );

      let clusterData: Record<string, any>;
      try {
        clusterData = JSON.parse(clusterOutput.stdout);
      } catch (e) {
        debug("describe-cluster output %O", clusterOutput);
        debug(e);
        throw e;
      }

      if (clusterData["cluster"]) {
        const awsEksCluster = new SiEntity({ entityType: "awsEksCluster" });
        awsEksCluster.name = clusterData["cluster"]["name"];
        awsEksCluster.addOpSet({
          op: OpType.Set,
          source: OpSource.Inferred,
          path: ["name"],
          value: awsEksCluster.name,
          system: "baseline",
        });
        awsEksCluster.addOpSet({
          op: OpType.Set,
          source: OpSource.Inferred,
          path: ["kubernetesVersion"],
          value: clusterData["cluster"]["version"],
          system: "baseline",
        });

        const awsEks = new SiEntity({ entityType: "awsEks" });
        awsEks.name = clusterData["cluster"]["name"];

        const kubernetesCluster = new SiEntity({
          entityType: "kubernetesCluster",
        });
        kubernetesCluster.name = clusterData["cluster"]["name"];

        response.discovered.push({
          entity: kubernetesCluster,
          configures: [
            {
              entity: awsEks,
              configures: [{ entity: awsEksCluster, configures: [] }],
            },
          ],
        });
      }
    }
  }
  return response;
}

export default { syncResource, discover };
