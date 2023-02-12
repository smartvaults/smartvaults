use std::fmt::Display;

use bdk::descriptor::{
    policy::{PkOrF, SatisfiableItem},
    Policy,
};

const INDENT_PX_UNIT: u32 = 20;

#[derive(Clone, PartialEq, Properties)]
pub struct PolicyNodeProps {
    pub node: Policy,
    pub depth: u32,
}

impl PolicyNodeProps {
    pub fn indent_style(&self) -> String {
        (0..self.depth).map(|_| " ").collect::<String>();
        // format!("margin-left: {}px", self.depth * INDENT_PX_UNIT)
    }

    pub fn id(&self) -> &str {
        &self.node.id
    }

    pub fn description(&self) -> String {
        match &self.node.item {
            SatisfiableItem::EcdsaSignature(key) => format!("ECDSA Sig of {}", display_key(key)),
            SatisfiableItem::SchnorrSignature(key) => {
                format!("Schnorr Sig of {}", display_key(key))
            }
            SatisfiableItem::Sha256Preimage { hash } => {
                format!("SHA256 Preimage of {}", hash.to_string())
            }
            SatisfiableItem::Hash256Preimage { hash } => {
                format!("Double-SHA256 Preimage of {}", hash.to_string())
            }
            SatisfiableItem::Ripemd160Preimage { hash } => {
                format!("RIPEMD160 Preimage of {}", hash.to_string())
            }
            SatisfiableItem::Hash160Preimage { hash } => {
                format!("Double-RIPEMD160 Preimage of {}", hash.to_string())
            }
            SatisfiableItem::AbsoluteTimelock { value } => {
                format!("Absolute Timelock of {}", value.to_string())
            }
            SatisfiableItem::RelativeTimelock { value } => {
                format!("Relative Timelock of {}", value.to_string())
            }
            SatisfiableItem::Multisig { keys, threshold } => {
                format!("{} of {} MultiSig:", threshold, keys.len())
            }
            SatisfiableItem::Thresh { items, threshold } => {
                format!("{} of {} Threshold:", threshold, items.len())
            }
        }
    }
}

pub struct DisplayKey<'a>(&'a PkOrF);

impl<'a> Display for DisplayKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", display_key(self.0))
    }
}

fn display_key(key: &PkOrF) -> String {
    // TODO: Use aliases
    match key {
        PkOrF::Pubkey(pk) => format!("<pk:{}>", pk.to_string()),
        PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{}>", pk.to_string()),
        PkOrF::Fingerprint(f) => format!("<fingerprint:{}>", f.to_string()),
    }
}

pub enum PolicyNodeMsg {}

pub struct PolicyNode {}

impl Component for PolicyNode {
    type Message = PolicyNodeMsg;
    type Properties = PolicyNodeProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        // let node_label = format!("{}", props.id(), props.description());

        let chooser_html: Html = match &props.node.item {
            SatisfiableItem::Multisig { keys, .. } => {
                let key_iter = keys.iter().enumerate().map(|(index, key)| -> Html {
                    let selection = props.selection.clone();
                    let parent_id = props.id().to_string();

                    html! { <MultiSigNode {selection} {parent_id} {index} pk_or_f={key.clone()} /> }
                });
                html! { for key_iter }
            }
            SatisfiableItem::Thresh { items, .. } => {
                let item_iter = items.iter().enumerate().map(|(index, node)| -> Html {
                    let selection = props.selection.clone();

                    let parent_id = props.id().to_string();
                    let node = PolicyNodeProps {
                        selection: props.selection.clone(),
                        node: node.clone(),
                        depth: props.depth + 1,
                    };

                    html! { <ThresholdNode {selection} {parent_id} {index} {node}/> }
                });
                html! { for item_iter }
            }
            _ => {
                html! {}
            }
        };

        html! {
            <div style = { props.indent_style() }>
                <label>{ props.description() }</label>
                <br/>
                { chooser_html }
            </div>
        }
    }
}

#[derive(PartialEq, Properties)]
pub struct MultiSigProps {
    pub selection: Selection,

    pub parent_id: String,
    pub index: usize,
    pub pk_or_f: PkOrF,
}

pub struct MultiSigNode(bool);

impl Component for MultiSigNode {
    type Message = Event;
    type Properties = MultiSigProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self(false)
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let props = ctx.props();
        let value: String = msg.target_unchecked_into::<HtmlInputElement>().value();

        self.0 = !self.0;
        if self.0 {
            log::info!("select {}", value);
            props.selection.select(props.parent_id.clone(), props.index);
        } else {
            log::info!("deselect {}", value);
            props
                .selection
                .deselect(props.parent_id.clone(), props.index);
        }

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();

        let input_id = || format!("check-{}-{}", props.parent_id, props.index);
        let label = display_key(&props.pk_or_f);
        log::info!("creating label: {}", label);

        let onchange = ctx.link().callback(|e: Event| e);

        html! {
            <div class="form-check">
                <input class="form-check-input" type="checkbox" value={ input_id() } id={ input_id() } {onchange}/>
                <label class="form-check-label" for={ input_id() }> { label } </label>
            </div>
        }
    }
}

#[derive(PartialEq, Properties)]
pub struct ThresholdProps {
    pub selection: Selection,

    pub parent_id: String,
    pub index: usize,
    pub node: PolicyNodeProps,
}

pub struct ThresholdNode(bool);

impl Component for ThresholdNode {
    type Message = Event;
    type Properties = ThresholdProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self(false)
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let props = ctx.props();
        let value: String = msg.target_unchecked_into::<HtmlInputElement>().value();

        self.0 = !self.0;
        if self.0 {
            log::info!("select {}", value);
            props.selection.select(props.parent_id.clone(), props.index);
        } else {
            log::info!("deselect {}", value);
            props
                .selection
                .deselect(props.parent_id.clone(), props.index);
        }

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        let node_props = props.node.clone();

        let input_id = || format!("check-{}-{}", props.parent_id, props.index);
        let label = props.node.id();

        let selection = props.selection.clone();

        let onchange = ctx.link().callback(|e: Event| e);

        html! {
            <div class="form-check">
                <input class="form-check-input" type="checkbox" value={ input_id() } id={ input_id() } {onchange}/>
                <label class="form-check-label" for={ input_id() }> { label } </label>
                <PolicyNode {selection} node={node_props.node} depth={node_props.depth}/>
            </div>
        }
    }
}
