-- Add timestamp columns to labels table
ALTER TABLE labels 
ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Remove the default value to ensure all new records specify timestamps explicitly
ALTER TABLE labels 
ALTER COLUMN created_at DROP DEFAULT,
ALTER COLUMN updated_at DROP DEFAULT;

-- Create trigger for updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger on labels table
CREATE TRIGGER update_labels_updated_at 
BEFORE UPDATE ON labels 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();