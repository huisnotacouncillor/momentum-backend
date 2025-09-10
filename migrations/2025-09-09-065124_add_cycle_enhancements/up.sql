-- Add description field to cycles table
ALTER TABLE cycles ADD COLUMN description TEXT;

-- Add goal field to cycles table for cycle objectives
ALTER TABLE cycles ADD COLUMN goal TEXT;

-- Add updated_at timestamp to cycles table
ALTER TABLE cycles ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Create index for better performance on cycle queries
CREATE INDEX idx_cycles_status ON cycles(status);
CREATE INDEX idx_cycles_start_date ON cycles(start_date);
CREATE INDEX idx_cycles_end_date ON cycles(end_date);