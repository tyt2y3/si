use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};

use crate::diagram::DiagramResult;
use crate::socket::{SocketArity, SocketEdgeKind};
use crate::{DalContext, DiagramError, Node, NodePosition, SchemaVariant, StandardModel};

#[derive(
    AsRefStr,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Display,
    EnumIter,
    EnumString,
    Eq,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum SocketDirection {
    Input,
    Output,
    Bidirectional,
}

#[derive(
    AsRefStr,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Display,
    EnumIter,
    EnumString,
    Eq,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum NodeSide {
    Left,
    Right,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocketView {
    id: String,
    label: String,
    #[serde(rename = "type")]
    ty: String,
    direction: SocketDirection,
    max_connections: Option<usize>,
    is_required: Option<bool>,
    node_side: NodeSide,
}

impl SocketView {
    pub async fn list(
        ctx: &DalContext,
        node: &Node,
        schema_variant: &SchemaVariant,
    ) -> DiagramResult<Vec<Self>> {
        let node_id = *node.id();
        let node_id: i64 = node_id.into();
        Ok(schema_variant
            .sockets(ctx)
            .await?
            .into_iter()
            .filter(|socket| socket.name() != "includes")
            .map(|socket| {
                let socket_id = *socket.id();
                let socket_id: i64 = socket_id.into();
                Self {
                    id: format!("{}-{}", node_id, socket_id),
                    label: socket.name().to_owned(),
                    ty: socket.name().to_owned(),
                    // Note: it's not clear if this mapping is correct, and there is no backend support for bidirectional sockets for now
                    direction: match socket.edge_kind() {
                        SocketEdgeKind::ConfigurationOutput => SocketDirection::Output,
                        _ => SocketDirection::Input,
                    },
                    max_connections: match socket.arity() {
                        SocketArity::Many => None,
                        SocketArity::One => Some(1),
                    },
                    is_required: Some(socket.required()),
                    node_side: match socket.edge_kind() {
                        SocketEdgeKind::ConfigurationOutput => NodeSide::Right,
                        _ => NodeSide::Left,
                    },
                }
            })
            .collect())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GridPoint {
    x: isize,
    y: isize,
}

impl GridPoint {
    pub fn x(&self) -> isize {
        self.x
    }

    pub fn y(&self) -> isize {
        self.y
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagramNodeView {
    id: String,
    #[serde(rename = "type")]
    ty: Option<String>,
    title: String,
    subtitle: Option<String>,
    content: Option<String>,
    sockets: Option<Vec<SocketView>>,
    position: GridPoint,
    color: Option<String>,
}

impl DiagramNodeView {
    pub async fn new(
        ctx: &DalContext,
        node: &Node,
        position: &NodePosition,
        schema_variant: &SchemaVariant,
    ) -> DiagramResult<Self> {
        let component = node
            .component(ctx)
            .await?
            .ok_or(DiagramError::ComponentNotFound)?;
        Ok(Self {
            id: node.id().to_string(),
            ty: None,
            title: schema_variant
                .schema(ctx)
                .await?
                .ok_or(DiagramError::SchemaNotFound)?
                .name()
                .to_owned(),
            subtitle: component
                .find_value_by_json_pointer(ctx, "/root/si/name")
                .await?,
            content: None,
            sockets: Some(SocketView::list(ctx, node, schema_variant).await?),
            position: GridPoint {
                x: position.x().parse()?,
                y: position.y().parse()?,
            },
            color: schema_variant
                .color()
                .map(|color_int| format!("#{color_int:x}")),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn position(&self) -> &GridPoint {
        &self.position
    }
}
