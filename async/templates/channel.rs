pub mod options {
  use super::*;

  {{#each protocol.classes as |class| ~}}
  {{#each class.methods as |method| ~}}
  {{#each_argument method.arguments as |argument| ~}}
  {{#unless argument_is_value ~}}
  #[derive(Clone, Debug, Default, PartialEq)]
  pub struct {{camel class.name}}{{camel method.name}}Options {
    {{#each_flag argument as |flag| ~}}
    pub {{snake flag.name}}: Boolean,
    {{/each_flag ~}}
  }

  {{/unless ~}}
  {{/each_argument ~}}
  {{/each ~}}
  {{/each ~}}
}

use options::*;

#[derive(Debug)]
pub enum Reply {
  {{#each protocol.classes as |class| ~}}
  {{#each class.methods as |method| ~}}
  {{#if method.synchronous ~}}
  Awaiting{{camel class.name}}{{camel method.name}}Ok(RequestId{{#each method.metadata.state as |state| ~}}, {{state.type}}{{/each ~}}),
  {{/if ~}}
  {{/each ~}}
  {{/each ~}}
}

impl Channel {
  pub(crate) fn receive_method(&self, method: AMQPClass) -> Result<(), Error> {
    match method {
      {{#each protocol.classes as |class| ~}}
      {{#unless class.metadata.skip ~}}
      {{#each class.methods as |method| ~}}
      {{#if method.is_reply ~}}
      AMQPClass::{{camel class.name}}(protocol::{{snake class.name}}::AMQPMethod::{{camel method.name}}(m)) => self.receive_{{snake class.name false}}_{{snake method.name false}}(m),
      {{/if ~}}
      // FIXME: dedupe
      {{#if method.metadata.can_be_received ~}}
      AMQPClass::{{camel class.name}}(protocol::{{snake class.name}}::AMQPMethod::{{camel method.name}}(m)) => self.receive_{{snake class.name false}}_{{snake method.name false}}(m),
      {{/if ~}}
      {{/each ~}}
      {{/unless ~}}
      {{/each ~}}
      m => {
        error!("the client should not receive this method: {:?}", m);
        Err(ErrorKind::InvalidMethod(m).into())
      }
    }
  }

  {{#each protocol.classes as |class| ~}}
  {{#unless class.metadata.skip ~}}
  {{#each class.methods as |method| ~}}
  pub fn {{snake class.name false}}_{{snake method.name false}}(&self{{#each_argument method.arguments as |argument| ~}}{{#if argument_is_value ~}}{{#unless argument.force_default ~}}, {{snake argument.name}}: {{#if (use_str_ref argument.type) ~}}&str{{else}}{{argument.type}}{{/if ~}}{{/unless ~}}{{else}}, options: {{camel class.name}}{{camel method.name}}Options{{/if ~}}{{/each_argument ~}}{{#each method.metadata.extra_args as |arg| ~}}, {{arg.name}}: {{arg.type}}{{/each ~}}) -> Result<Option<{{#if method.metadata.end_hook.return_type ~}}{{method.metadata.end_hook.return_type}}{{else}}RequestId{{/if ~}}>, Error> {
    {{#if method.metadata.channel_init ~}}
    if !self.status.is_initializing() {
    {{else}}
    if !self.status.is_connected() {
    {{/if ~}}
      return Err(ErrorKind::NotConnected.into());
    }

    {{#each_argument method.arguments as |argument| ~}}
    {{#unless argument_is_value ~}}
    let {{camel class.name}}{{camel method.name}}Options {
      {{#each_flag argument as |flag| ~}}
      {{snake flag.name}},
      {{/each_flag ~}}
    } = options;
    {{/unless ~}}
    {{/each_argument ~}}

    let method = AMQPClass::{{camel class.name}}(protocol::{{snake class.name}}::AMQPMethod::{{camel method.name}} (protocol::{{snake class.name}}::{{camel method.name}} {
      {{#each_argument method.arguments as |argument| ~}}
      {{#if argument_is_value ~}}
      {{#unless argument.force_default ~}}
      {{snake argument.name}}: {{snake argument.name}}{{#if (use_str_ref argument.type) ~}}.to_string(){{/if ~}},
      {{/unless ~}}
      {{else}}
      {{#each_flag argument as |flag| ~}}
      {{snake flag.name}},
      {{/each_flag ~}}
      {{/if ~}}
      {{/each_argument ~}}
    }));

    self.send_method_frame(method);

    {{#if method.metadata.end_hook ~}}
    {{#if method.metadata.end_hook.return_type ~}}let end_hook_ret = {{/if ~}}self.on_{{snake class.name false}}_{{snake method.name false}}_sent({{#each method.metadata.end_hook.params as |param| ~}}{{#unless @first ~}}, {{/unless ~}}{{param}}{{/each ~}});
    {{/if ~}}

    Ok(
      {{#if method.synchronous ~}}
      {{#if (method_has_flag method "nowait") ~}}
      if nowait {
        None
      } else {{/if ~}}{
        let request_id = self.request_id.next();
        self.replies.register_pending(self.id, Reply::Awaiting{{camel class.name}}{{camel method.name}}Ok(request_id{{#each method.metadata.state as |state| ~}}, {{state.name}}{{#if state.use_str_ref ~}}.to_string(){{/if ~}}{{/each ~}}));
        Some(request_id)
      }
      {{else}}
      {{#if method.metadata.end_hook.return_type ~}}
      end_hook_ret
      {{else}}
      None
      {{/if}}
      {{/if ~}}
    )
  }

  {{#if method.is_reply ~}}
  fn receive_{{snake class.name false}}_{{snake method.name false}}(&self, {{#if method.arguments ~}}method{{else}}_{{/if ~}}: protocol::{{snake class.name}}::{{camel method.name}}) -> Result<(), Error> {
    {{#if method.metadata.channel_init ~}}
    if !self.status.is_initializing() {
    {{else}}
    if !self.status.is_connected() {
    {{/if ~}}
      return Err(ErrorKind::NotConnected.into());
    }

    match self.replies.next() {
      Some(Reply::Awaiting{{camel class.name}}{{camel method.name}}(request_id{{#each method.metadata.state as |state| ~}}, {{state.name}}{{/each ~}})) => {
        self.requests.finish(request_id, true);
        {{#if method.arguments ~}}
        self.on_{{snake class.name false}}_{{snake method.name false}}_received(method{{#if method.metadata.uses_request_id ~}}, request_id{{/if ~}}{{#each method.metadata.state as |state| ~}}, {{state.name}}{{/each ~}})
        {{else}}
        {{#if method.metadata.received_hook ~}}
        self.on_{{snake class.name false}}_{{snake method.name false}}_received({{#each method.metadata.received_hook.params as |param| ~}}{{#unless @first ~}}, {{/unless ~}}{{param}}{{/each ~}})
        {{else}}
        Ok(())
        {{/if ~}}
        {{/if ~}}
      },
      _ => {
        self.set_error()?;
        Err(ErrorKind::UnexpectedReply.into())
      },
    }
  }
  {{/if ~}}
  {{#if method.metadata.can_be_received ~}}
  fn receive_{{snake class.name false}}_{{snake method.name false}}(&self, method: protocol::{{snake class.name}}::{{camel method.name}}) -> Result<(), Error> {
    if !self.status.is_connected() {
      return Err(ErrorKind::NotConnected.into());
    }
    self.on_{{snake class.name false}}_{{snake method.name false}}_received(method)
  }
  {{/if ~}}
  {{/each ~}}
  {{/unless ~}}
  {{/each ~}}
}
