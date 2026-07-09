ALTER TABLE public.channel_messages
    ADD COLUMN search_vector tsvector;

CREATE INDEX index_channel_messages_on_search_vector
    ON public.channel_messages USING gin (search_vector);

CREATE FUNCTION public.update_channel_message_search_vector()
RETURNS trigger AS $$
BEGIN
    NEW.search_vector := to_tsvector('english', COALESCE(NEW.body, ''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_channel_messages_search_vector
    BEFORE INSERT OR UPDATE ON public.channel_messages
    FOR EACH ROW
    EXECUTE FUNCTION public.update_channel_message_search_vector();

UPDATE public.channel_messages
SET search_vector = to_tsvector('english', COALESCE(body, ''));
