// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{Agent, AgentResult, BoxError, CompositeAgentError, EmptyCompositeAgentError};

/// A static group of heterogeneous Agents run by one owner.
pub struct CompositeAgent {
    agents: Vec<Box<dyn Agent>>,
    role_name: String,
    index: usize,
}

impl CompositeAgent {
    /// Creates a non-empty composite.
    pub fn new(agents: Vec<Box<dyn Agent>>) -> Result<Self, EmptyCompositeAgentError> {
        if agents.is_empty() {
            return Err(EmptyCompositeAgentError);
        }
        let mut role_name = String::from("[");
        for (index, agent) in agents.iter().enumerate() {
            if index != 0 {
                role_name.push(',');
            }
            role_name.push_str(agent.role_name());
        }
        role_name.push(']');
        Ok(Self {
            agents,
            role_name,
            index: 0,
        })
    }
}

impl Agent for CompositeAgent {
    fn role_name(&self) -> &str {
        &self.role_name
    }

    fn on_start(&mut self) -> Result<(), BoxError> {
        self.index = 0;
        let errors: Vec<_> = self
            .agents
            .iter_mut()
            .filter_map(|a| a.on_start().err())
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Box::new(CompositeAgentError::new("on_start", errors)))
        }
    }

    fn do_work(&mut self) -> AgentResult {
        let mut work_count = 0i32;
        while self.index < self.agents.len() {
            let index = self.index;
            self.index += 1;
            work_count = work_count.wrapping_add(self.agents[index].do_work()?);
        }
        self.index = 0;
        Ok(work_count)
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.index = 0;
        let errors: Vec<_> = self
            .agents
            .iter_mut()
            .filter_map(|a| a.on_close().err())
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Box::new(CompositeAgentError::new("on_close", errors)))
        }
    }
}
