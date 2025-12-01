---
title: "Analyze Project Structure"
description: "Deep dive into codebase organization"
categories: ["Explore codebase", "onboarding"]
secondary_tag: "Code Analysis"
author: "kodegen-team"
verified: true
votes: 85
parameters:
  - name: "project_path"
    description: "Path to the project to analyze"
    param_type: "string"
    required: false
    default: "."
---

I'll help you understand this project's structure.

{% if project_path != "." %}
Analyzing project at: **{{ project_path }}**
{% else %}
Analyzing current directory: **{{ project_path }}**
{% endif %}

I'll examine:
- Directory structure and organization
- Key files and their purposes
- Dependencies and build configuration
- Entry points and main modules

Let me start by exploring {{ project_path }}...
