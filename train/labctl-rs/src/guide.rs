// guide.rs — step-by-step walkthrough state, parsed from docs/tui-guide.txt
//
// Format (line markers):
//   # STEP n: Title        — starts a step
//   @choice id | Name      — a method option (id links it to later @variant blocks)
//   @variant id            — method-specific text shown in later steps
//   @action open-connect   — offers a "jump to Connect" action for step 1
//   # END                  — closes the current @choice/@variant block
//
// The chosen method (choice id) is remembered by the app; when a later step
// has a @variant with that id, its text is appended to the step body.
use std::fs;

pub struct Choice {
    pub id: String,
    pub name: String,
    pub body: String,
}

pub struct Step {
    pub title: String,
    pub body: String,                     // shown when the step opens (intro/commands)
    pub choices: Vec<Choice>,             // optional methods (pivot steps)
    pub variants: Vec<(String, String)>,  // method-specific addenda (id, text)
    pub has_connect_action: bool,
}

pub struct Guide {
    pub steps: Vec<Step>,
}

pub const GUIDE_PATH: &str = "/home/lab/tunnriculum/train/docs/tui-guide.txt";

impl Guide {
    pub fn load() -> Result<Guide, String> {
        let candidates = [
            GUIDE_PATH,
            "train/docs/tui-guide.txt",
            "docs/tui-guide.txt",
            "../docs/tui-guide.txt",
        ];
        for path in candidates {
            if let Ok(raw) = fs::read_to_string(path) {
                return Guide::load_from(&raw);
            }
        }
        Err(format!("could not read {}", GUIDE_PATH))
    }

    pub fn load_from(raw: &str) -> Result<Guide, String> {
        let mut steps: Vec<Step> = Vec::new();
        // where body lines currently go
        enum Sink { Step, Choice(usize), Variant(usize) }
        let mut sink = Sink::Step;
        let mut cur: Option<Step> = None;

        for line in raw.lines() {
            if let Some(title) = line.strip_prefix("# STEP ") {
                if let Some(s) = cur.take() { steps.push(s); }
                cur = Some(Step {
                    title: title.trim().to_string(),
                    body: String::new(),
                    choices: Vec::new(),
                    variants: Vec::new(),
                    has_connect_action: false,
                });
                sink = Sink::Step;
            } else if let Some(rest) = line.strip_prefix("@choice ") {
                let rest = rest.trim();
                let (id, name) = match rest.split_once('|') {
                    Some((id, name)) => (id.trim().to_string(), name.trim().to_string()),
                    None => (rest.to_string(), rest.to_string()),
                };
                if let Some(step) = cur.as_mut() {
                    step.choices.push(Choice { id, name, body: String::new() });
                    sink = Sink::Choice(step.choices.len() - 1);
                }
            } else if let Some(vid) = line.strip_prefix("@variant ") {
                if let Some(step) = cur.as_mut() {
                    step.variants.push((vid.trim().to_string(), String::new()));
                    sink = Sink::Variant(step.variants.len() - 1);
                }
            } else if line.trim() == "@action open-connect" {
                if let Some(step) = cur.as_mut() {
                    step.has_connect_action = true;
                }
            } else if line.trim() == "# END" {
                sink = Sink::Step;
            } else if let Some(step) = cur.as_mut() {
                match &mut sink {
                    Sink::Choice(i) => {
                        let ch = &mut step.choices[*i];
                        if !ch.body.is_empty() || !line.trim().is_empty() {
                            ch.body.push_str(line);
                            ch.body.push('\n');
                        }
                    }
                    Sink::Variant(i) => {
                        let body = &mut step.variants[*i].1;
                        if !body.is_empty() || !line.trim().is_empty() {
                            body.push_str(line);
                            body.push('\n');
                        }
                    }
                    Sink::Step => {
                        if !step.body.is_empty() || !line.trim().is_empty() {
                            step.body.push_str(line);
                            step.body.push('\n');
                        }
                    }
                }
            }
        }
        if let Some(s) = cur.take() { steps.push(s); }
        if steps.is_empty() {
            return Err("guide has no steps".to_string());
        }
        Ok(Guide { steps })
    }

    pub fn len(&self) -> usize { self.steps.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# STEP 1: Get connected
Intro line one.

@action open-connect
# STEP 2: Pick a method
Prepare the payload here.

@choice burp | A. Burp Suite
Burp body line.

@choice curl | B. curl
curl body line.

@variant burp
Burp variant text.
# END
# END
# STEP 3: The end
Final body.
";

    #[test]
    fn parses_steps_titles_and_bodies() {
        let g = Guide::load_from(SAMPLE).unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g.steps[0].title, "1: Get connected");
        assert!(g.steps[0].body.contains("Intro line one."));
        assert_eq!(g.steps[2].title, "3: The end");
    }

    #[test]
    fn choices_parse_id_and_name() {
        let g = Guide::load_from(SAMPLE).unwrap();
        let s2 = &g.steps[1];
        assert_eq!(s2.choices.len(), 2);
        assert_eq!(s2.choices[0].id, "burp");
        assert_eq!(s2.choices[0].name, "A. Burp Suite");
        assert!(s2.choices[1].body.contains("curl body line."));
        // choice bodies sink correctly: burp's body must NOT leak into curl's
        assert!(!s2.choices[1].body.contains("Burp body"));
    }

    #[test]
    fn end_marker_returns_sink_to_step_body() {
        let g = Guide::load_from(SAMPLE).unwrap();
        // after "# END", following lines go back to the step body
        assert!(!g.steps[1].body.contains("Final body."));
        assert!(g.steps[2].body.contains("Final body."));
    }

    #[test]
    fn variants_and_actions_attach() {
        let g = Guide::load_from(SAMPLE).unwrap();
        let s2 = &g.steps[1];
        assert_eq!(s2.variants.len(), 1);
        assert_eq!(s2.variants[0].0, "burp");
        assert!(s2.variants[0].1.contains("Burp variant text."));
        assert!(g.steps[0].has_connect_action);
        assert!(!s2.has_connect_action);
    }

    #[test]
    fn empty_guide_errors() {
        assert!(Guide::load_from("nothing here").is_err());
    }
}
