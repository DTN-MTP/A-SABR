


fn main() {



    // Nodes and contacts can be implemented with different management techniques
    // This is implemented in A-SABR with a "base" holding a "manager" part
    // For performance, this is translated by templating (BaseClass<TemplateClass>):
    // - the Node structure is templated by concrete manager that implements the NodeManager "trait" (interface/abtract class)
    // - the Contact structure is templated by concrete manager that implements the ContactManager "trait" (interface/abtract class)

    // The Rust compiler requires the parser to know the exact types of the Nodes and Contacts being parsed (base + manager)
    // As a consequence, the parsing functions (ION, TVG, A-SABR) are templated by the Manager Types.

    // The parsers are "class methods" (no need for an initialization of a struct) of IONContactPlan, TVGUtilContactPlan, ASABRContactPlan
    // And they return a tuples of this type : (Vec<Node<SomeManager_1>>, Vec<Contact<SomeManger_2>>), with SomeManager_1 and SomeManager_2
    // being concrete implementation of NodeManager and ContactManager traits, respectively

    // Step 1: retrieve the nodes and contacts for the ION format

    //let (nodes, contacts) = IONContactPlan::parse::<NoManagement, SegmentationManager>( )


}
