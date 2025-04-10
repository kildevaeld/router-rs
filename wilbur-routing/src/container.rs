use wilbur_container::modules::BuildContext;

pub trait RouterBuildContext: BuildContext {
    type Body;
}
