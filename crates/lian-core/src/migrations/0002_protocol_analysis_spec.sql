-- A protocol may carry a machine-readable predefined analysis specification
-- (exposure/outcome/lag/exclusions as accepted by analysis.run) so its result
-- can be produced exactly as pre-registered, linked via analysis_results.protocol_id.
ALTER TABLE research_protocols ADD COLUMN analysis_spec TEXT;
